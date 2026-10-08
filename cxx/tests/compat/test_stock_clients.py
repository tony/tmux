"""Exercise declared stock command modes and admission against the owned C++ daemon."""

import array
import fcntl
import os
import selectors
import signal
import socket
import struct
import subprocess
import termios
from contextlib import suppress
from pathlib import Path

import pytest
import tmux_cxx

from tests.conftest import fixture_environment


def identify_command_peer(
    peer: socket.socket, identification_flags: tuple[int, int] | None = None
) -> None:
    """Pass command descriptors and optional flags, then finish legacy identification."""
    with open(os.devnull, "r+b") as terminal:
        for kind in (104, 110):
            frame = struct.pack("<IIII", kind, 16 | 0x10000, 8, os.getpid())
            sent = peer.sendmsg(
                [frame],
                [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [terminal.fileno()]))],
            )
            peer.sendall(frame[sent:])
    if identification_flags is not None:
        message_type, flags = identification_flags
        flag_format = "<Q" if message_type == 111 else "<I"
        payload = struct.pack(flag_format, flags)
        peer.sendall(
            struct.pack("<IIII", message_type, 16 + len(payload), 8, os.getpid()) + payload
        )
    peer.sendall(struct.pack("<IIII", 106, 16, 8, os.getpid()))


def identify_terminal_peer(peer: socket.socket, terminal: int) -> None:
    """Pass one owned TTY twice and finish legacy ordinary-client identification."""
    for kind in (104, 110):
        frame = struct.pack("<IIII", kind, 16 | 0x10000, 8, os.getpid())
        sent = peer.sendmsg(
            [frame], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array("i", [terminal]))]
        )
        peer.sendall(frame[sent:])
    peer.sendall(struct.pack("<IIII", 106, 16, 8, os.getpid()))


def command_frame(arguments: list[bytes]) -> bytes:
    """Encode legacy command arguments for admission-order tests without invoking a tmux binary."""
    payload = struct.pack("<I", len(arguments)) + b"\0".join(arguments) + b"\0"
    return struct.pack("<IIII", 200, 16 + len(payload), 8, os.getpid()) + payload


def receive_stock_frame(peer: socket.socket) -> tuple[int, bytes]:
    """Read one bounded legacy stock frame from a raw test peer."""
    frame = bytearray()
    while len(frame) < 16:
        fragment = peer.recv(16 - len(frame))
        assert fragment, "stock peer closed before its frame header"
        frame.extend(fragment)
    message, length, version, _ = struct.unpack("<IIII", frame)
    assert version == 8
    assert 16 <= length <= 65536
    while len(frame) < length:
        fragment = peer.recv(length - len(frame))
        assert fragment, "stock peer closed before its complete frame"
        frame.extend(fragment)
    return message, bytes(frame[16:])


def read_control_block(descriptor: int) -> bytes:
    """Read one bounded stock control block through its closing guard."""
    output = bytearray()
    with selectors.DefaultSelector() as ready:
        ready.register(descriptor, selectors.EVENT_READ)
        while True:
            assert ready.select(timeout=0.7), "stock control block was not delivered"
            fragment = os.read(descriptor, 65536)
            assert fragment, "stock control client closed before its block ended"
            output.extend(fragment)
            assert len(output) <= 1048576
            if any(line.startswith((b"%end ", b"%error ")) for line in output.splitlines()):
                return bytes(output)


def read_control_until(descriptor: int, needle: bytes) -> bytes:
    """Read bounded stock control records until one expected byte sequence arrives."""
    output = bytearray()
    with selectors.DefaultSelector() as ready:
        ready.register(descriptor, selectors.EVENT_READ)
        while needle not in output:
            assert ready.select(timeout=0.7), f"control client did not emit {needle!r}"
            fragment = os.read(descriptor, 65536)
            assert fragment, "stock control client closed before its expected record"
            output.extend(fragment)
            assert len(output) <= 1048576
    return bytes(output)


def read_terminal_until(descriptor: int, needle: bytes) -> bytes:
    """Read bounded PTY output until one rendered token appears."""
    output = bytearray()
    with selectors.DefaultSelector() as ready:
        ready.register(descriptor, selectors.EVENT_READ)
        while needle not in output:
            assert ready.select(timeout=0.7), f"terminal did not render {needle!r}"
            output.extend(os.read(descriptor, 65536))
            assert len(output) <= 1048576
    return bytes(output)


def discard_terminal_output(descriptor: int) -> None:
    """Discard currently readable PTY bytes without waiting for later output."""
    blocking = os.get_blocking(descriptor)
    os.set_blocking(descriptor, False)
    try:
        while True:
            if not os.read(descriptor, 65536):
                break
    except BlockingIOError:
        pass
    finally:
        os.set_blocking(descriptor, blocking)


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_registered_stock_command_subset(server: str, fixture: str) -> None:
    """Unmodified legacy and modern clients mutate one C++ session through registered commands."""
    environment = fixture_environment()
    client = os.environ[fixture]
    base = [client, "-N", "-f", "/dev/null", "-S", server]
    subprocess.run(
        [*base, "new-session", "-d", "-s", "stock", "/bin/cat"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    with tmux_cxx.ServerConnection(server) as observer:
        original_pane = observer.sessions()[0].pane
    subprocess.run(
        [*base, "split-window", "-h", "-t", f"%{original_pane}", "/bin/cat"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "select-pane", "-t", f"%{original_pane}"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "set-option", "-g", "prefix", "C-a"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "set", "-g", "prefix2", "C-x"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "set-option", "-t", "stock", "prefix", "C-z"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "set-buffer", "-b", "shared", "stock-buffer-token\n"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "paste-buffer", "-b", "shared", "-t", f"%{original_pane}"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    subprocess.run(
        [*base, "rename-session", "-t", "stock", "renamed"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    result = subprocess.run(
        [*base, "list-sessions", "-F", "#{session_name}:#{pane_id}"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.sessions()[0]
        assert b"stock-buffer-token" in observer.wait_pane_output(
            original_pane, b"stock-buffer-token", 500
        )
        assert observer.read_paste_buffer("shared") == b"stock-buffer-token\n"
        assert session.name == "renamed"
        assert result.stdout == f"renamed:%{session.pane}\n".encode()
        panes = observer.panes(session.window)
        assert [(pane.columns, pane.active) for pane in panes] == [(40, True), (39, False)]
    subprocess.run(
        [*base, "delete-buffer", "-b", "shared"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_attach_routes_terminal_and_preserves_session(server: str, fixture: str) -> None:
    """A stock client sees every split pane, routes active input, and restores on detach."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session(
            "interactive", "printf 'left-pane-ready\\n'; exec /bin/cat"
        )
        right_pane = observer.split_pane(
            session.pane,
            tmux_cxx.PaneSplitOrientation.LEFT_RIGHT,
            "printf 'right-pane-ready\\n'; exec /bin/cat",
        )
        observer.wait_pane_output(session.pane, b"left-pane-ready", 500)
        observer.wait_pane_output(right_pane.id, b"right-pane-ready", 500)
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 10, 40, 0, 0))
        inspection = os.dup(slave)
        original = termios.tcgetattr(inspection)
        client = subprocess.Popen(
            [
                os.environ[fixture],
                "-N",
                "-f",
                "/dev/null",
                "-S",
                server,
                "attach-session",
                "-t",
                "interactive",
            ],
            stdin=slave,
            stdout=slave,
            stderr=subprocess.PIPE,
            env=fixture_environment(),
        )
        assert client.stderr is not None
        client_stderr = client.stderr
        os.close(slave)
        try:
            rendered = read_terminal_until(master, b"right-pane-ready")
            assert b"\x1b[?1049h" in rendered
            assert b"left-pane-ready" in rendered
            assert b"|" in rendered
            attached = observer.clients()
            assert len(attached) == 1
            assert attached[0].session == session.id
            assert attached[0].identified and attached[0].has_terminal
            compatibility = observer.stock_client_compatibility(attached[0].id)
            expected_profile = (
                "imsg-16-linux-x86_64"
                if fixture == "TMUX_CXX_STOCK_CLIENT"
                else "imsg-32-linux-x86_64"
            )
            assert compatibility.profile == expected_profile
            assert compatibility.release == "" and compatibility.release_uncertain
            assert compatibility.direction == "stock_client_to_cxx_server"
            assert compatibility.mode == "interactive"
            assert compatibility.tier == "general"
            assert compatibility.support == "limited"
            assert compatibility.verification == "tested_subset"
            assert "attach-session" in compatibility.commands
            assert "delete-buffer" in compatibility.commands
            assert "paste-buffer" in compatibility.commands
            assert "select-pane" in compatibility.commands
            assert "set" in compatibility.commands
            assert "set-buffer" in compatibility.commands
            assert "set-option" in compatibility.commands
            assert "split-window" in compatibility.commands
            assert compatibility.quirks == []
            assert compatibility.evidence and compatibility.limitations
            window = observer.window(session.window)
            assert (window.rows, window.columns) == (10, 40)

            discard_terminal_output(master)
            fcntl.ioctl(inspection, termios.TIOCSWINSZ, struct.pack("HHHH", 12, 42, 0, 0))
            client.send_signal(signal.SIGWINCH)
            read_terminal_until(master, b"\x1b[2J")
            window = observer.window(session.window)
            assert (window.rows, window.columns) == (12, 42)

            discard_terminal_output(master)
            observer.select_pane(session.pane)
            read_terminal_until(master, b"\x1b[2J")
            os.write(master, b"left-client-input\n")
            assert b"left-client-input" in read_terminal_until(master, b"left-client-input")
            observer.wait_pane_output(session.pane, b"left-client-input", 500)
            detached = observer.detach_client(attached[0].id)
            assert detached.session is None
            assert detached.detached_from == "interactive"
            status = client.wait(timeout=0.7)
            assert status == 0, client_stderr.read().decode(errors="replace")
        finally:
            if client.poll() is None:
                client.kill()
                client.wait(timeout=0.15)
        assert observer.clients() == []
        restored = termios.tcgetattr(inspection)
        os.close(master)
        os.close(inspection)
        assert restored[:4] == original[:4]
        assert observer.sessions()[0].id == session.id


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_exit_detaches_client_only(server: str, fixture: str) -> None:
    """A terminating stock client completes EXITING while its session remains retained."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("client-exit", "printf 'attached\\n'; exec /bin/cat")
        master, slave = os.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 0, 0, 0, 0))
        inspection = os.dup(slave)
        original = termios.tcgetattr(inspection)
        client = subprocess.Popen(
            [
                os.environ[fixture],
                "-N",
                "-f",
                "/dev/null",
                "-S",
                server,
                "attach-session",
                "-t",
                "client-exit",
            ],
            stdin=slave,
            stdout=slave,
            stderr=subprocess.PIPE,
            env=fixture_environment(),
        )
        assert client.stderr is not None
        client_stderr = client.stderr
        os.close(slave)
        try:
            read_terminal_until(master, b"attached")
            attached = observer.clients()[0]
            assert attached.session == session.id
            client.send_signal(signal.SIGTERM)
            status = client.wait(timeout=0.7)
            assert status == 1, client_stderr.read().decode(errors="replace")
        finally:
            if client.poll() is None:
                client.kill()
                client.wait(timeout=0.15)
        assert observer.clients() == []
        restored = termios.tcgetattr(inspection)
        os.close(master)
        os.close(inspection)
        assert restored[:4] == original[:4]
        assert observer.sessions()[0].id == session.id


def test_zero_sized_stock_terminal_reports_named_quirk(server: str) -> None:
    """Raw stock identification exposes the declared 24x80 adjustment only when it is applied."""
    master, slave = os.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 0, 0, 0, 0))
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
            peer.connect(server)
            identify_terminal_peer(peer, slave)
            with tmux_cxx.ServerConnection(server) as observer:
                clients = observer.clients()
                assert len(clients) == 1
                assert (clients[0].rows, clients[0].columns) == (24, 80)
                compatibility = observer.stock_client_compatibility(clients[0].id)
                assert compatibility.quirks == ["zero_terminal_dimensions_use_defaults"]
                assert "zero rows use 24 and zero columns use 80" in compatibility.limitations
    finally:
        os.close(master)
        os.close(slave)


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_control_mode_executes_registered_commands(server: str, fixture: str) -> None:
    """Pinned stock clients exchange guarded commands without ordinary-client attachment."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("control", "/bin/cat")
        client = subprocess.Popen(
            [
                os.environ[fixture],
                "-N",
                "-f",
                "/dev/null",
                "-S",
                server,
                "-C",
                "list-sessions",
                "-F",
                "#{session_name}",
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=fixture_environment(),
        )
        assert client.stdin is not None
        assert client.stdout is not None
        assert client.stderr is not None
        try:
            initial = read_control_block(client.stdout.fileno())
            initial_lines = initial.splitlines()
            assert initial_lines[0].startswith(b"%begin ")
            assert initial_lines[0].split()[-1] == b"0"
            assert b"control" in initial_lines
            assert initial_lines[-1].startswith(b"%end ")
            assert initial_lines[-1].split()[1:] == initial_lines[0].split()[1:]
            control_client = observer.clients()[0]
            assert control_client.identified and not control_client.has_terminal
            compatibility = observer.stock_client_compatibility(control_client.id)
            assert compatibility.mode == "control"
            assert compatibility.tier == "general"
            assert compatibility.support == "limited"
            assert compatibility.verification == "tested_subset"
            assert compatibility.quirks == ["unattached_control_channel_stays_open"]
            assert "rename-session" in compatibility.commands
            assert any(
                "one registered command per line" in value for value in compatibility.limitations
            )

            client.stdin.write(b"rename-session -t control renamed\n")
            client.stdin.flush()
            renamed_notice = f"%session-renamed ${session.id} renamed".encode()
            renamed = read_control_until(client.stdout.fileno(), renamed_notice).splitlines()
            assert renamed[0].startswith(b"%begin ")
            assert renamed[0].split()[-1] == b"1"
            end_index = next(
                index for index, line in enumerate(renamed) if line.startswith(b"%end ")
            )
            assert renamed[end_index].split()[1:] == renamed[0].split()[1:]
            assert renamed.index(renamed_notice) > end_index

            client.stdin.write(b"list-sessions -F #{session_name}\n")
            client.stdin.flush()
            listed = read_control_block(client.stdout.fileno()).splitlines()
            assert listed[0].split()[-1] == b"1"
            assert b"renamed" in listed
            assert listed[-1].startswith(b"%end ")
            assert listed[-1].split()[1:] == listed[0].split()[1:]

            client.stdin.write(b"new-session -d -s forbidden /bin/cat ; list-sessions\n")
            client.stdin.flush()
            rejected = read_control_block(client.stdout.fileno()).splitlines()
            assert rejected[0].split()[-1] == b"1"
            assert rejected[-1].startswith(b"%error ")
            assert [session.name for session in observer.sessions()] == ["renamed"]

            client.stdin.write(b"\n")
            client.stdin.flush()
            assert client.wait(timeout=0.7) == 0, client.stderr.read().decode(errors="replace")
            assert b"%exit" in client.stdout.read()
        finally:
            if client.poll() is None:
                client.kill()
                client.wait(timeout=0.15)
        assert [session.name for session in observer.sessions()] == ["renamed"]


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_control_attachment_streams_new_pane_output(server: str, fixture: str) -> None:
    """A control attachment starts at live output and follows explicit session switches."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session(
            "control-output", "printf 'prior-output-token\\n'; exec /bin/cat"
        )
        observer.wait_pane_output(session.pane, b"prior-output-token", 500)
        second = observer.create_session("control-second", "/bin/cat")
        client = subprocess.Popen(
            [
                os.environ[fixture],
                "-N",
                "-f",
                "/dev/null",
                "-S",
                server,
                "-C",
                "attach-session",
                "-t",
                "control-output",
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=fixture_environment(),
        )
        assert client.stdin is not None
        assert client.stdout is not None
        assert client.stderr is not None
        try:
            session_changed = f"%session-changed ${session.id} control-output".encode()
            initial = read_control_until(client.stdout.fileno(), session_changed)
            assert initial.index(b"%end ") < initial.index(session_changed)
            assert b"prior-output-token" not in initial
            assert b"%sessions-changed" not in initial
            attached = observer.clients()
            assert len(attached) == 1
            assert attached[0].session == session.id
            assert not attached[0].has_terminal

            observer.send_pane_input(session.pane, b"control-output-token\n")
            output = read_control_until(client.stdout.fileno(), b"control-output-token")
            pane_prefix = f"%output %{session.pane} ".encode()
            assert pane_prefix in output
            assert b"control-output-token\\015\\012" in output

            observer.rename_session(session.id, "control-renamed")
            renamed_notice = f"%session-renamed ${session.id} control-renamed".encode()
            read_control_until(client.stdout.fileno(), renamed_notice)

            client.stdin.write(b"attach-session -t control-second\n")
            client.stdin.flush()
            second_session_changed = f"%session-changed ${second.id} control-second".encode()
            switched = read_control_until(client.stdout.fileno(), second_session_changed)
            assert switched.index(b"%end ") < switched.index(second_session_changed)

            observer.send_pane_input(session.pane, b"old-session-token\n")
            observer.send_pane_input(second.pane, b"second-session-token\n")
            second_output = read_control_until(client.stdout.fileno(), b"second-session-token")
            second_pane_prefix = f"%output %{second.pane} ".encode()
            assert second_pane_prefix in second_output
            assert b"old-session-token" not in second_output

            observer.kill_session(second.id)
            read_control_until(client.stdout.fileno(), b"%sessions-changed\n")
            detached = observer.clients()
            assert len(detached) == 1
            assert detached[0].session is None
            assert detached[0].detached_from == "control-second"
            client.stdin.write(b"list-sessions -F #{session_name}\n")
            client.stdin.flush()
            listed = read_control_block(client.stdout.fileno())
            assert b"control-renamed" in listed
            assert b"control-second" not in listed

            client.stdin.write(b"\n")
            client.stdin.flush()
            assert client.wait(timeout=0.7) == 0, client.stderr.read().decode(errors="replace")
        finally:
            if client.poll() is None:
                client.kill()
                client.wait(timeout=0.15)


def test_stock_command_before_identification_has_no_effects(server: str) -> None:
    """An unprofiled command closes conservatively before creating a session."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.settimeout(0.7)
        peer.connect(server)
        peer.sendall(command_frame([b"new-session", b"-d", b"-s", b"forbidden", b"/bin/cat"]))
        assert peer.recv(1) == b""
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions() == []


def test_malformed_identification_metadata_closes_before_effects(server: str) -> None:
    """A declared NUL-terminated identification field is validated before command admission."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.connect(server)
        peer.sendall(struct.pack("<IIII", 101, 20, 8, os.getpid()) + b"dumb")
        with selectors.DefaultSelector() as closed:
            closed.register(peer, selectors.EVENT_READ)
            assert closed.select(timeout=0.35), "malformed identification peer remained open"
        assert peer.recv(1) == b""
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions() == []


def test_late_stock_control_flags_cannot_bypass_admission(server: str) -> None:
    """Late identification flags cannot change the admitted mode or create a session."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.settimeout(0.7)
        peer.connect(server)
        identify_command_peer(peer)
        peer.sendall(
            struct.pack("<IIIIQ", 111, 24, 8, os.getpid(), 0x2000)
            + command_frame([b"new-session", b"-d", b"-s", b"forbidden", b"/bin/cat"])
        )
        with peer.makefile("rb") as response:
            response.read()
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions() == []


@pytest.mark.parametrize(
    ("flags_message_type", "client_flags"),
    [
        pytest.param(100, 0x2000 | 0x4000, id="echo-disabled-control"),
        pytest.param(100, 0x2000 | 0x4000000, id="control-no-output"),
        pytest.param(111, 0x2000 | 0x100000000, id="control-pause-after"),
        pytest.param(111, 0x2000 | 0x200000000, id="control-wait-exit"),
    ],
)
def test_unsupported_stock_control_flags_close_before_effects(
    server: str, flags_message_type: int, client_flags: int
) -> None:
    """Unsupported control transports and flow options close before command effects."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.settimeout(0.7)
        peer.connect(server)
        identify_command_peer(peer, (flags_message_type, client_flags))
        peer.sendall(command_frame([b"new-session", b"-d", b"-s", b"forbidden", b"/bin/cat"]))
        with suppress(ConnectionResetError):
            assert peer.recv(1) == b""
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions() == []


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_echo_disabled_stock_control_closes_before_effects(server: str, fixture: str) -> None:
    """Unmodified `-CC` clients are rejected before their initial command can mutate state."""
    master, slave = os.openpty()
    client = subprocess.Popen(
        [
            os.environ[fixture],
            "-N",
            "-f",
            "/dev/null",
            "-S",
            server,
            "-CC",
            "new-session",
            "-d",
            "-s",
            "forbidden",
            "/bin/cat",
        ],
        stdin=slave,
        stdout=slave,
        stderr=subprocess.PIPE,
        env=fixture_environment(),
    )
    os.close(slave)
    try:
        assert client.wait(timeout=0.7) != 0
    finally:
        if client.poll() is None:
            client.kill()
            client.wait(timeout=0.15)
        os.close(master)
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions() == []


def test_stock_command_cannot_replace_pending_output(server: str) -> None:
    """A second command awaiting the first output acknowledgement cannot mutate server state."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("retained", "/bin/cat")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
            peer.settimeout(0.7)
            peer.connect(server)
            identify_command_peer(peer)
            peer.sendall(
                command_frame([b"list-sessions"])
                + command_frame([b"new-session", b"-d", b"-s", b"forbidden", b"/bin/cat"])
            )
            with peer.makefile("rb") as response:
                response.read()
        assert [value.id for value in observer.sessions()] == [session.id]


def test_attached_stock_client_rejects_later_command_before_effects(server: str) -> None:
    """An attached client cannot return to command admission and mutate server state."""
    master, slave = os.openpty()
    try:
        with tmux_cxx.ServerConnection(server) as observer:
            retained = observer.create_session("retained", "/bin/cat")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
                peer.settimeout(0.7)
                peer.connect(server)
                identify_terminal_peer(peer, slave)
                peer.sendall(command_frame([b"attach-session", b"-t", b"retained"]))
                assert receive_stock_frame(peer) == (207, b"")
                assert observer.clients()[0].session == retained.id

                peer.sendall(
                    command_frame([b"new-session", b"-d", b"-s", b"forbidden", b"/bin/cat"])
                )
                with selectors.DefaultSelector() as closed:
                    closed.register(peer, selectors.EVENT_READ)
                    assert closed.select(timeout=0.35), "invalid attached command was retained"
                assert peer.recv(1) == b""
            assert [session.name for session in observer.sessions()] == ["retained"]
    finally:
        os.close(master)
        os.close(slave)


def test_incomplete_stock_identification_expires(server: str) -> None:
    """An idle stock peer expires within its identification bound while native requests progress."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        peer.connect(server)
        with tmux_cxx.ServerConnection(server) as observer:
            assert observer.sessions() == []
        with selectors.DefaultSelector() as closed:
            closed.register(peer, selectors.EVENT_READ)
            assert closed.select(timeout=0.35), "incomplete stock identification was retained"
        assert peer.recv(1) == b""


@pytest.mark.parametrize(
    ("fixture", "profile", "release"),
    [
        ("TMUX_CXX_STOCK_CLIENT", "legacy", "3.4"),
        ("TMUX_CXX_MODERN_STOCK_CLIENT", "modern", "3.7"),
    ],
)
def test_cpp_client_runs_against_owned_stock_server(
    fixture: str, profile: str, release: str
) -> None:
    """The C++ reverse adapter verifies release evidence and captures a real command result."""
    environment = fixture_environment()
    stock_tmux = os.environ[fixture]
    socket_path = Path(os.environ["TMPDIR"]) / f"stock-{os.getpid()}-{profile}.sock"
    session = f"reverse {profile}"
    assert not socket_path.exists()
    endpoint = [stock_tmux, "-f", "/dev/null", "-S", str(socket_path)]
    subprocess.run(
        [*endpoint, "new-session", "-d", "-s", session, "/bin/cat"],
        env=environment,
        check=True,
        capture_output=True,
        timeout=0.9,
    )
    try:
        mismatched_profile = "modern" if profile == "legacy" else "legacy"
        rejected = subprocess.run(
            [
                os.environ["TMUX_CXX_STOCK_SERVER_CLIENT"],
                mismatched_profile,
                str(socket_path),
                release,
                session,
            ],
            env=environment,
            capture_output=True,
            timeout=2.5,
        )
        assert rejected.returncode != 0

        result = subprocess.run(
            [
                os.environ["TMUX_CXX_STOCK_SERVER_CLIENT"],
                profile,
                str(socket_path),
                release,
                session,
            ],
            env=environment,
            capture_output=True,
            timeout=2.5,
        )
        assert result.returncode == 0, result.stderr.decode(errors="replace")
    finally:
        subprocess.run(
            [*endpoint, "kill-server"],
            env=environment,
            check=False,
            capture_output=True,
            timeout=0.9,
        )
