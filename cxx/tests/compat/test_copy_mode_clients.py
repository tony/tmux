"""Verify real attached tmux clients share frozen pane modes and resume program input."""

import fcntl
import os
import selectors
import signal
import struct
import subprocess
import termios
from collections.abc import Generator
from contextlib import contextmanager

import pytest
import tmux_cxx

from tests.compat.test_stock_clients import discard_terminal_output, read_terminal_until
from tests.conftest import stock_fixture_environment


def drain_copy_terminal(terminal: int) -> bytes:
    """Collect currently readable attached-terminal bytes without waiting for future output."""
    output = bytearray()
    with selectors.DefaultSelector() as ready:
        ready.register(terminal, selectors.EVENT_READ)
        while ready.select(timeout=0):
            fragment = os.read(terminal, 65536)
            if not fragment:
                break
            output.extend(fragment)
            assert len(output) <= 1048576
    return bytes(output)


def read_copy_view(
    terminal: int,
    observer: tmux_cxx.ServerConnection,
    pane: int,
    *,
    column: int | None = None,
    selected: bool | None = None,
    size: tuple[int, int] | None = None,
) -> tmux_cxx.CopyModeSnapshot:
    """Wait for rendered copy updates and committed cursor, selection, or dimensions."""
    received = 0
    with selectors.DefaultSelector() as ready:
        ready.register(terminal, selectors.EVENT_READ)
        while True:
            assert ready.select(timeout=0.7), (
                "attached client did not render the expected copy state"
            )
            fragment = os.read(terminal, 65536)
            assert fragment, "attached client closed before the expected copy state"
            received += len(fragment)
            assert received <= 1048576
            snapshot = observer.copy_mode(pane)
            mode = snapshot.mode
            if mode is None:
                continue
            if column is not None and mode.cursor.column != column:
                continue
            if selected is not None and (mode.selection is not None) != selected:
                continue
            if size is not None and (mode.rows, mode.columns) != size:
                continue
            return snapshot


@contextmanager
def attached_copy_terminal(
    server: str, fixture: str
) -> Generator[tuple[int, int, subprocess.Popen[bytes]]]:
    """Own one stock attachment and verify terminal restoration after a normal detach."""
    master, slave = os.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 6, 24, 0, 0))
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
            "attach",
            "-t",
            "attached-copy",
        ],
        stdin=slave,
        stdout=slave,
        stderr=subprocess.PIPE,
        env=stock_fixture_environment(os.environ[fixture]),
    )
    os.close(slave)
    try:
        yield master, inspection, client
    finally:
        if client.poll() is None:
            client.kill()
            client.wait(timeout=0.15)
        elif client.returncode == 0:
            assert termios.tcgetattr(inspection) == original
        if client.stderr is not None:
            client.stderr.close()
        os.close(master)
        os.close(inspection)


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_attached_copy_keys_resize_and_resume_program(server: str, fixture: str) -> None:
    """Two stock clients share copy mode, resize it, copy cells, and resume program input."""
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("attached-copy", "/bin/cat")
        observer.send_pane_input(session.pane, b"abc\n")
        observer.wait_pane_output(session.pane, b"abc\r\n")
        other_fixture = (
            "TMUX_CXX_MODERN_STOCK_CLIENT"
            if fixture == "TMUX_CXX_STOCK_CLIENT"
            else "TMUX_CXX_STOCK_CLIENT"
        )
        with (
            attached_copy_terminal(server, fixture) as (master, inspection, client),
            attached_copy_terminal(server, other_fixture) as (other_master, _, other_client),
        ):
            read_terminal_until(master, b"abc")
            read_terminal_until(other_master, b"abc")
            discard_terminal_output(other_master)
            discard_terminal_output(master)
            os.write(master, b"\x02[")
            captured = read_copy_view(master, observer, session.pane, size=(6, 24))
            assert captured.mode is not None
            assert (captured.mode.rows, captured.mode.columns) == (6, 24)
            shared = read_copy_view(other_master, observer, session.pane)
            assert shared.display_revision == captured.display_revision
            observer.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.HISTORY_START)
            observer.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.LINE_START)
            observer.begin_copy_selection(session.pane)
            read_copy_view(master, observer, session.pane, column=0, selected=True)
            discard_terminal_output(master)
            os.write(master, b"\x1b[C")
            selected = read_copy_view(master, observer, session.pane, column=1, selected=True)
            assert selected.mode is not None and selected.mode.selection is not None
            assert selected.mode.selection.end.column == 1
            discard_terminal_output(other_master)
            os.write(other_master, b"\x1b[C")
            selected = read_copy_view(other_master, observer, session.pane, column=2, selected=True)
            assert selected.mode is not None and selected.mode.selection is not None
            assert selected.mode.selection.end.column == 2
            observer.send_pane_input(session.pane, b"hidden-copy-output\n")
            observer.wait_pane_output(session.pane, b"hidden-copy-output\r\n")
            frozen = observer.copy_mode(session.pane)
            assert frozen.display_revision == selected.display_revision
            assert frozen.mode is not None and frozen.mode.lines == selected.mode.lines
            assert observer.read_pane_text(session.pane).revision > frozen.mode.source_revision
            assert b"hidden-copy-output" not in drain_copy_terminal(master)
            assert b"hidden-copy-output" not in drain_copy_terminal(other_master)
            assert client.poll() is None and len(observer.clients()) == 2
            discard_terminal_output(master)
            os.write(master, b"\x17")
            read_terminal_until(master, b"hidden-copy-output")
            assert observer.copy_mode(session.pane).mode is None
            assert observer.read_paste_buffer(observer.paste_buffers()[0].name) == b"ab"
            discard_terminal_output(master)
            os.write(master, b"\x02]\n")
            observer.wait_pane_output(session.pane, b"ab\r\n")
            read_terminal_until(master, b"ab")

            observer.set_window_option(session.window, "mode-keys", "vi")
            discard_terminal_output(master)
            os.write(master, b"\x02[")
            read_copy_view(master, observer, session.pane)
            observer.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.HISTORY_START)
            observer.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.LINE_START)
            observer.begin_copy_selection(session.pane)
            read_copy_view(master, observer, session.pane, column=0, selected=True)
            discard_terminal_output(master)
            os.write(master, b"l")
            vi_selection = read_copy_view(master, observer, session.pane, column=1, selected=True)
            assert vi_selection.mode is not None and vi_selection.mode.selection is not None
            assert vi_selection.mode.selection.end.column == 1
            source_revision = vi_selection.mode.source_revision
            os.write(other_master, b"\x02d")
            other_client.wait(timeout=0.7)
            assert other_client.returncode == 0
            assert observer.copy_mode(session.pane).mode is not None
            assert len(observer.clients()) == 1
            discard_terminal_output(master)
            fcntl.ioctl(inspection, termios.TIOCSWINSZ, struct.pack("HHHH", 8, 30, 0, 0))
            client.send_signal(signal.SIGWINCH)
            resized = read_copy_view(master, observer, session.pane, selected=False, size=(8, 30))
            assert resized.mode is not None and resized.mode.selection is None
            assert (resized.mode.rows, resized.mode.columns) == (8, 30)
            assert resized.mode.source_revision == source_revision
            assert resized.display_revision > vi_selection.display_revision
            observer.begin_copy_selection(session.pane)
            read_copy_view(master, observer, session.pane, selected=True)
            discard_terminal_output(master)
            os.write(master, b"\x1b")
            cleared = read_copy_view(master, observer, session.pane, selected=False)
            assert cleared.mode is not None and cleared.mode.selection is None
            discard_terminal_output(master)
            os.write(master, b"qresumed-after-copy\n")
            read_terminal_until(master, b"resumed-after-copy")
            observer.wait_pane_output(session.pane, b"resumed-after-copy\r\n")
            assert observer.copy_mode(session.pane).mode is None
            assert client.poll() is None
            os.write(master, b"\x02d")
            client.wait(timeout=0.7)
            assert client.returncode == 0
            assert observer.sessions()[0].id == session.id
