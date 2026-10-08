"""Reject untrusted native reply metadata through the compiled C++ connection."""

import socket
import struct
from concurrent.futures import ThreadPoolExecutor

import pytest
import tmux_cxx


def native_bytes(value: bytes) -> bytes:
    """Encode counted native bytes without changing Unicode or binary content."""
    return struct.pack("<I", len(value)) + value


def read_native_frame(peer: socket.socket) -> tuple[int, bytes]:
    """Receive one complete bounded frame under the socket's declared timeout."""
    request = bytearray()
    total = 12
    operation = -1
    while len(request) < total:
        fragment = peer.recv(total - len(request))
        assert fragment, "native peer truncated its frame"
        request.extend(fragment)
        if len(request) == 12:
            magic, version, operation, length = struct.unpack("<4sHHI", request)
            assert (magic, version) == (b"TMXD", 1)
            assert length <= 1048576
            total += length
    return operation, bytes(request[12:])


def test_native_invalid_name_rejected_before_effects(server: str) -> None:
    """Raw native create and rename requests reject invalid text without changing server state."""

    def request(operation: int, arguments: bytes) -> bytes:
        """Exchange an owned daemon request and require its corresponding reply."""
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
            peer.settimeout(0.7)
            peer.connect(server + ".native")
            peer.sendall(struct.pack("<4sHHI", b"TMXD", 1, operation, len(arguments)) + arguments)
            selected, reply = read_native_frame(peer)
            assert selected == operation
            return reply

    negotiated = request(0, native_bytes(b"") + struct.pack("<H", 1))
    length = struct.unpack_from("<I", negotiated)[0]
    owner = negotiated[4 : 4 + length]
    assert length == 32
    assert struct.unpack_from("<H", negotiated, 4 + length)[0] == 0
    created = request(1, native_bytes(owner) + native_bytes(b"\xff") + native_bytes(b"/bin/cat"))
    assert struct.unpack_from("<H", created, 4 + length)[0] == 3
    with tmux_cxx.ServerConnection(server) as connection:
        assert connection.sessions() == []
        session = connection.create_session("valid", "/bin/cat")
        assert session.id == 1
        renamed = request(
            3, native_bytes(owner) + struct.pack("<Q", session.id) + native_bytes(b"\xff")
        )
        assert struct.unpack_from("<H", renamed, 4 + length)[0] == 3
        unchanged = connection.sessions()[0]
        assert (unchanged.name, unchanged.revision) == (session.name, session.revision)
        assert connection.window(session.window).name == session.name


@pytest.mark.parametrize(
    ("owner", "code", "message", "suffix"),
    [
        (b"", 11, b"unsupported", b""),
        (b"invalid", 11, b"unsupported", b""),
        (b"0" * 32, 65535, b"unsupported", b""),
        (b"0" * 32, 11, b"unsupported", b"extra"),
        (b"0" * 32, 11, b"\xff", b""),
    ],
    ids=["missing-owner", "invalid-owner", "unknown-error", "excess-error-bytes", "invalid-utf8"],
)
def test_native_reply_rejects_untrusted_envelope(
    tmp_path_factory: pytest.TempPathFactory, owner: bytes, code: int, message: bytes, suffix: bytes
) -> None:
    """Untrusted reply envelopes cannot pin an owner before later valid traffic is admitted."""
    endpoint = tmp_path_factory.mktemp("native_peer") / "sock"
    payload = (
        struct.pack("<I", len(owner))
        + owner
        + struct.pack("<HI", code, len(message))
        + message
        + suffix
    )
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.settimeout(0.7)
        listener.bind(str(endpoint) + ".native")
        listener.listen(1)

        def respond(envelope: bytes) -> None:
            """Read a complete handshake before returning this test's selected reply envelope."""
            with listener.accept()[0] as peer:
                peer.settimeout(0.7)
                operation, request = read_native_frame(peer)
                assert (operation, request) == (0, b"\0" * 4 + struct.pack("<H", 1))
                peer.sendall(struct.pack("<4sHHI", b"TMXD", 1, operation, len(envelope)) + envelope)

        with ThreadPoolExecutor(max_workers=1) as worker:
            response = worker.submit(respond, payload)
            with tmux_cxx.ServerConnection(str(endpoint)) as connection:
                with pytest.raises(tmux_cxx.TmuxError) as failure:
                    connection.sessions()
                assert failure.value.code == "protocol"
                assert failure.value.operation == "session.list"
                response.result(timeout=0.7)
                valid_owner = b"1" * 32
                valid_message = b"declared unsupported operation"
                valid = (
                    struct.pack("<I", len(valid_owner))
                    + valid_owner
                    + struct.pack("<HI", 11, len(valid_message))
                    + valid_message
                )
                response = worker.submit(respond, valid)
                with pytest.raises(tmux_cxx.TmuxError) as declared:
                    connection.sessions()
                assert declared.value.code == "unsupported"
                assert declared.value.server_instance == valid_owner.decode()
                response.result(timeout=0.7)


@pytest.mark.parametrize(
    ("operation", "identity", "effect", "capability", "malformed"),
    [
        (
            1,
            "session.create",
            1,
            1,
            struct.pack("<4Q", 1, 2, 3, 4) + native_bytes(b"\xff") + struct.pack("<Q", 1),
        ),
        (2, "session.list", 0, 1, b"\0"),
        (
            2,
            "session.list",
            0,
            1,
            struct.pack("<I4Q", 1, 1, 2, 3, 4) + native_bytes(b"\xff") + struct.pack("<Q", 1),
        ),
        (3, "session.rename", 1, 2, b"extra"),
        (4, "session.kill", 1, 1, b"extra"),
        (5, "pane.send_input", 2, 8, b"extra"),
        (6, "pane.read_output", 0, 16, b"\0"),
        (7, "pane.wait_output", 0, 32, b"\0"),
        (8, "window.link", 1, 4, b"\0"),
        (9, "window.unlink", 1, 4, b"extra"),
        (10, "window.list_links", 0, 4, b"\0"),
        (11, "window.read", 0, 4, b"\0"),
        (
            11,
            "window.read",
            0,
            4,
            struct.pack("<2Q", 1, 2) + native_bytes(b"\xff") + struct.pack("<Q", 1),
        ),
        (
            25,
            "session.observe",
            0,
            2048,
            struct.pack("<I", 0) + native_bytes(b"3" * 32) + struct.pack("<Q", 0),
        ),
    ],
    ids=[
        "create-invalid-utf8",
        "list-truncated",
        "list-invalid-utf8",
        "rename-excess",
        "kill-excess",
        "send-input-excess",
        "read-output-truncated",
        "wait-output-truncated",
        "link-truncated",
        "unlink-excess",
        "list-links-truncated",
        "window-truncated",
        "window-invalid-utf8",
        "observation-wrong-owner",
    ],
)
def test_native_success_payload_rejection_retains_context(
    tmp_path_factory: pytest.TempPathFactory,
    operation: int,
    identity: str,
    effect: int,
    capability: int,
    malformed: bytes,
) -> None:
    """Reject malformed operation payloads with protocol errors and accepted ownership context."""
    endpoint = tmp_path_factory.mktemp("native_payload") / "sock"
    owner = b"2" * 32
    description = (
        struct.pack("<H", operation)
        + native_bytes(identity.encode())
        + struct.pack("<HQ", effect, capability)
    )
    handshake = struct.pack("<HIQI", 1, 1048576, capability, 1) + description
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.settimeout(0.7)
        listener.bind(str(endpoint) + ".native")
        listener.listen(1)

        def respond() -> None:
            """Negotiate the selected operation, then send its malformed successful reply."""
            for selected, payload in ((0, handshake), (operation, malformed)):
                with listener.accept()[0] as peer:
                    peer.settimeout(0.7)
                    received, arguments = read_native_frame(peer)
                    assert received == selected
                    if selected == 0:
                        assert arguments == b"\0" * 4 + struct.pack("<H", 1)
                    else:
                        assert arguments.startswith(native_bytes(owner))
                    envelope = native_bytes(owner) + struct.pack("<HI", 0, 0) + payload
                    peer.sendall(
                        struct.pack("<4sHHI", b"TMXD", 1, selected, len(envelope)) + envelope
                    )

        with ThreadPoolExecutor(max_workers=1) as worker:
            response = worker.submit(respond)
            with tmux_cxx.ServerConnection(str(endpoint)) as connection:
                with pytest.raises(tmux_cxx.TmuxError) as failure:
                    match identity:
                        case "session.create":
                            connection.create_session("name", "/bin/cat")
                        case "session.list":
                            connection.sessions()
                        case "session.rename":
                            connection.rename_session(1, "name")
                        case "session.kill":
                            connection.kill_session(1)
                        case "pane.send_input":
                            connection.send_pane_input(1, b"input")
                        case "pane.read_output":
                            connection.read_pane_output(1)
                        case "pane.wait_output":
                            connection.wait_pane_output(1, b"needle", 500)
                        case "window.link":
                            connection.link_window(1, 2, 0)
                        case "window.unlink":
                            connection.unlink_window(1)
                        case "window.list_links":
                            connection.window_links(1)
                        case "window.read":
                            connection.window(1)
                        case "session.observe":
                            connection.observe_sessions()
                        case _:
                            raise AssertionError("unknown controlled operation fixture")
                assert failure.value.code == "protocol"
                assert failure.value.operation == identity
                assert failure.value.server_instance == owner.decode()
            response.result(timeout=0.7)
