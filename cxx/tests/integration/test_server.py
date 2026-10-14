"""Critical persistent server behavior exercised through the built Python extension."""

import hashlib
import os
import re
import selectors
import signal
import time
from contextlib import suppress
from pathlib import Path

import pytest
import tmux_cxx

from tests.conftest import OwnedServer


def test_session_pty_and_stale_identity(server: str) -> None:
    """Renaming preserves identity, PTY input produces bytes, and deletion rejects stale IDs."""
    connection = tmux_cxx.ServerConnection(server)
    session = connection.create_session("python", "/bin/sh")
    assert session.pane > 0
    connection.rename_session(session.id, "renamed")
    assert connection.sessions()[0].name == "renamed"
    connection.send_pane_input(session.pane, b"printf 'python-%s\\n' 'pty-token'\n")
    assert b"python-pty-token" in connection.wait_pane_output(
        session.pane, b"python-pty-token", 500
    )
    connection.kill_session(session.id)
    with pytest.raises(tmux_cxx.TmuxError):
        connection.rename_session(session.id, "stale")


def test_connection_disposal_preserves_session(server: str) -> None:
    """Closing one wrapper leaves its session available to another connection."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("persistent", "/bin/sh")
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions()[0].id == session.id


def test_windows_own_programs_and_session_selection(server: str) -> None:
    """Separate connections create a window, select its membership, and reject it after removal."""
    with (
        tmux_cxx.ServerConnection(server) as creator,
        tmux_cxx.ServerConnection(server) as observer,
    ):
        session = creator.create_session("windows", "/bin/cat")
        link = creator.create_window(session.id, 7, "tools:build.1", "/bin/cat")
        assert observer.sessions()[0].link == session.link
        assert [value.index for value in observer.window_links(session.id)] == [0, 7]
        window = observer.window(link.window)
        assert window.name == "tools:build.1"
        assert window.pane != session.pane
        selected = observer.select_window(link.id)
        assert selected.id == session.id
        assert selected.window == link.window
        assert selected.pane == window.pane
        assert creator.select_window(link.id).revision == selected.revision
        creator.send_pane_input(window.pane, b"new-window-pty\n")
        assert b"new-window-pty" in observer.wait_pane_output(window.pane, b"new-window-pty")
        with pytest.raises(tmux_cxx.TmuxError) as duplicate:
            creator.create_window(session.id, 7, "rejected", "/bin/cat")
        assert duplicate.value.code == "duplicate_index"
        observer.unlink_window(link.id)
        assert creator.sessions()[0].link == session.link
        with pytest.raises(tmux_cxx.TmuxError) as stale:
            creator.select_window(link.id)
        assert stale.value.code == "not_found"


def test_duplicate_rejection_preserves_state(server: str) -> None:
    """Rejecting a duplicate name preserves the existing session."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("existing", "/bin/sh")
        with pytest.raises(tmux_cxx.TmuxError):
            connection.create_session("existing", "/bin/sh")
        assert [s.id for s in connection.sessions()] == [session.id]


def test_window_mode_keys_are_inherited_and_shared(server: str) -> None:
    """Global defaults and window overrides remain shared across memberships and connections."""
    with (
        tmux_cxx.ServerConnection(server) as creator,
        tmux_cxx.ServerConnection(server) as observer,
    ):
        initial = creator.window_options(None)
        assert initial.window is None and initial.revision == 0
        assert initial.mode_keys == tmux_cxx.WindowModeKeys.EMACS
        assert re.fullmatch(r"[0-9a-f]{32}", initial.server_instance)
        first = creator.create_session("window-options", "/bin/cat")
        second = creator.create_session("window-options-linked", "/bin/cat")
        creator.link_window(first.window, second.id, 4)
        creator.set_window_option(None, "mode-keys", "vi")
        inherited = observer.window_options(first.window)
        assert inherited.window == first.window
        assert inherited.server_instance == initial.server_instance
        assert inherited.mode_keys == tmux_cxx.WindowModeKeys.VI and inherited.revision > 0
        assert observer.window_options(second.window).mode_keys == tmux_cxx.WindowModeKeys.VI
        creator.set_window_option(first.window, "mode-keys", "emacs")
        overridden = observer.window_options(first.window)
        assert overridden.mode_keys == tmux_cxx.WindowModeKeys.EMACS
        creator.set_window_option(first.window, "mode-keys", "emacs")
        assert observer.window_options(first.window).revision == overridden.revision
        creator.set_window_option(None, "mode-keys", "emacs")
        assert observer.window_options(first.window).revision == overridden.revision
        assert observer.window_options(second.window).mode_keys == tmux_cxx.WindowModeKeys.EMACS
        with pytest.raises(tmux_cxx.TmuxError) as invalid:
            creator.set_window_option(first.window, "mode-keys", "invalid")
        assert invalid.value.code == "invalid_argument"
        assert invalid.value.operation == "window.set_option"
        assert observer.window_options(first.window).revision == overridden.revision
        with pytest.raises(tmux_cxx.TmuxError) as unknown:
            creator.set_window_option(first.window, "prefix", "C-a")
        assert unknown.value.code == "invalid_argument"
        with pytest.raises(tmux_cxx.TmuxError) as zero:
            creator.window_options(0)
        assert zero.value.code == "invalid_argument"
        with pytest.raises(tmux_cxx.TmuxError) as missing:
            creator.window_options(99999)
        assert missing.value.code == "not_found" and missing.value.operation == "window.options"
        creator.kill_session(first.id)
        observer.kill_session(second.id)
        with pytest.raises(tmux_cxx.TmuxError) as stale:
            creator.window_options(first.window)
        assert stale.value.code == "not_found"
        with pytest.raises(tmux_cxx.TmuxError) as stale_set:
            creator.set_window_option(first.window, "mode-keys", "vi")
        assert stale_set.value.code == "not_found"
        assert stale_set.value.operation == "window.set_option"
    with pytest.raises(tmux_cxx.TmuxError) as closed:
        creator.set_window_option(None, "mode-keys", "vi")
    assert closed.value.code == "closed"
    assert closed.value.operation == "window.set_option"


def test_copy_mode_freezes_shared_display_and_copies_cells(server: str) -> None:
    """Frozen backing and shared selections survive program output and connection disposal."""
    with (
        tmux_cxx.ServerConnection(server) as creator,
        tmux_cxx.ServerConnection(server) as observer,
    ):
        session = creator.create_session("copy-mode", "/bin/cat")
        assert creator.copy_mode(session.pane).mode is None
        with pytest.raises(tmux_cxx.TmuxError) as zero:
            creator.copy_mode(0)
        assert zero.value.code == "invalid_argument"
        creator.send_pane_input(session.pane, b"abc\n")
        creator.wait_pane_output(session.pane, b"abc\r\n")
        creator.enter_copy_mode(session.pane)
        frozen = observer.copy_mode(session.pane)
        assert frozen.pane == session.pane and re.fullmatch(r"[0-9a-f]{32}", frozen.server_instance)
        assert frozen.mode is not None and frozen.mode.lines[0].startswith("abc")
        assert frozen.mode.mode_keys == frozen.mode.highlight_keys == tmux_cxx.WindowModeKeys.EMACS
        creator.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.HISTORY_START)
        creator.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.LINE_START)
        creator.begin_copy_selection(session.pane)
        creator.move_copy_cursor(session.pane, tmux_cxx.CopyCursorDirection.RIGHT, 2)
        selected = observer.copy_mode(session.pane)
        assert selected.mode is not None and selected.mode.selection is not None
        assert selected.mode.selection.anchor.column == 0
        assert selected.mode.selection.end.column == 2
        creator.move_copy_cursor(session.pane, tmux_cxx.CopyCursorDirection.RIGHT, 0)
        assert observer.copy_mode(session.pane).display_revision == selected.display_revision
        creator.enter_copy_mode(session.pane)
        assert observer.copy_mode(session.pane).display_revision == selected.display_revision
        creator.send_pane_input(session.pane, b"behind-copy-mode\n")
        creator.wait_pane_output(session.pane, b"behind-copy-mode\r\n")
        retained = observer.copy_mode(session.pane)
        assert retained.mode is not None
        assert retained.mode.source_revision == selected.mode.source_revision
        assert retained.mode.lines == selected.mode.lines
        assert retained.display_revision == selected.display_revision
        assert observer.read_pane_text(session.pane).revision > retained.mode.source_revision
        creator.set_window_option(session.window, "mode-keys", "vi")
        changed_keys = observer.copy_mode(session.pane)
        assert changed_keys.mode is not None
        assert changed_keys.mode.mode_keys == tmux_cxx.WindowModeKeys.VI
        assert changed_keys.mode.highlight_keys == tmux_cxx.WindowModeKeys.EMACS
        copied = creator.copy_selection(
            session.pane, tmux_cxx.CopySelectionDisposition.KEEP_SELECTION
        )
        assert copied is not None and copied.automatic and copied.byte_count == 3
        assert observer.read_paste_buffer(copied.name) == b"abc"
        creator.clear_copy_selection(session.pane)
        cleared = observer.copy_mode(session.pane)
        assert cleared.mode is not None and cleared.mode.selection is None
        creator.begin_copy_selection(session.pane)
        default_copy = creator.copy_selection(session.pane)
        assert default_copy is not None and observer.read_paste_buffer(default_copy.name) == b"c"
        cleared = observer.copy_mode(session.pane)
        assert cleared.mode is not None and cleared.mode.selection is None
        creator.scroll_copy_mode(session.pane, tmux_cxx.CopyScrollMotion.LINE_UP)
        creator.begin_copy_selection(session.pane)
        copied_cell = creator.copy_selection(
            session.pane, tmux_cxx.CopySelectionDisposition.CANCEL_MODE
        )
        assert copied_cell is not None and observer.read_paste_buffer(copied_cell.name) == b"c"
        assert observer.copy_mode(session.pane).mode is None
        creator.cancel_copy_mode(session.pane)
        creator.enter_copy_mode(session.pane)
        creator.close()
        assert observer.copy_mode(session.pane).mode is not None
        assert (
            observer.copy_selection(session.pane, tmux_cxx.CopySelectionDisposition.CANCEL_MODE)
            is None
        )
        assert observer.copy_mode(session.pane).mode is None
        observer.kill_session(session.id)
        with pytest.raises(tmux_cxx.TmuxError) as stale:
            observer.copy_mode(session.pane)
        assert stale.value.code == "not_found" and stale.value.operation == "pane.copy_mode"
    with pytest.raises(tmux_cxx.TmuxError) as closed:
        creator.enter_copy_mode(session.pane)
    assert closed.value.code == "closed" and closed.value.operation == "pane.enter_copy_mode"


def test_error_preserves_code_and_operation(server: str) -> None:
    """A missing session reports its stable classification and requested operation."""
    with tmux_cxx.ServerConnection(server) as connection:
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.rename_session(99999, "missing")
        assert getattr(failure.value, "code", None) == "not_found"
        assert getattr(failure.value, "operation", None) == "session.rename"


def test_oversized_arguments_preserve_error_and_state(server: str) -> None:
    """Oversized requests retain operation and server context without changing state."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("unchanged", "/bin/cat")
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.rename_session(session.id, "x" * 1048577)
        assert failure.value.code == "invalid_argument"
        assert failure.value.operation == "session.rename"
        assert failure.value.server_instance
        with pytest.raises(tmux_cxx.TmuxError) as combined:
            connection.create_session("x" * 524288, "y" * 524288)
        assert combined.value.code == "invalid_argument"
        assert combined.value.operation == "session.create"
        assert connection.sessions()[0].name == "unchanged"


def test_pane_bytes_preserve_invalid_utf8_and_nul(server: str) -> None:
    """Raw output preserves binary bytes instead of requiring valid Unicode."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("bytes", "printf '\\377\\000binary-end'; exec cat")
        output = connection.wait_pane_output(session.pane, b"binary-end", 500)
        assert b"\xff\x00binary-end" in output


def test_program_input_retains_nonblocking_write_remainder(server: str) -> None:
    """Queued program input delivers every byte while a nonblocking PTY accepts partial writes."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session(
            "queued",
            "stty raw -echo; printf 'queued-ready\\n'; "
            "head -c 65536 | sha256sum; printf 'queued-complete\\n'; exec cat",
        )
        connection.wait_pane_output(session.pane, b"queued-ready", 500)
        admitted = b"".join(index.to_bytes(4, "little") for index in range(16384))
        connection.send_pane_input(session.pane, admitted)
        output = connection.wait_pane_output(session.pane, b"queued-complete", 500)
        assert hashlib.sha256(admitted).hexdigest().encode() in output


def test_terminal_queries_and_text_share_retained_state(server: str) -> None:
    """A program receives cursor replies and bindings observe its pane's interpreted text."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session(
            "terminal",
            "stty raw -echo; printf '\\033[2;3H\\033[31;1m\\347\\225\\214\\033[6n'; "
            "reply=$(dd bs=1 count=6 2>/dev/null | od -An -tu1); "
            "printf 'terminal-query=%s:done' \"$reply\"; exec cat",
        )
        output = connection.wait_pane_output(session.pane, b":done", 500)
        assert b"27  91  50  59  53  82" in output
        view = connection.read_pane_text(session.pane)
        assert view.pane == session.pane
        assert re.fullmatch(r"[0-9a-f]{32}", view.server_instance)
        assert (view.rows, view.columns) == (24, 80)
        assert view.lines[1].startswith("  界terminal-query=")
        assert view.complete
        assert view.revision > 0
        assert view.input_open


def test_linked_window_survives_its_original_session(server: str) -> None:
    """Session membership removal preserves a shared window until its final link disappears."""
    with tmux_cxx.ServerConnection(server) as connection:
        first = connection.create_session("first", "/bin/cat")
        second = connection.create_session("second", "/bin/cat")
        link = connection.link_window(first.window, second.id, 7)
        assert (link.window, link.session, link.index) == (first.window, second.id, 7)
        assert [entry.index for entry in connection.window_links(second.id)] == [0, 7]
        renamed = connection.rename_window(first.window, "python:shared.1")
        assert (renamed.id, renamed.name) == (first.window, "python:shared.1")
        assert connection.window(link.window).name == "python:shared.1"
        assert connection.next_window(second.id).link == link.id
        assert connection.next_window(second.id).link == second.link
        assert connection.previous_window(second.id).link == link.id
        assert connection.previous_window(second.id).link == second.link
        with pytest.raises(tmux_cxx.TmuxError) as missing:
            connection.next_window(99999)
        assert (missing.value.code, missing.value.operation) == ("not_found", "window.next")
        with pytest.raises(tmux_cxx.TmuxError) as invalid:
            connection.rename_window(first.window, "invalid\n")
        assert (invalid.value.code, invalid.value.operation) == (
            "invalid_argument",
            "window.rename",
        )
        connection.kill_session(first.id)
        connection.unlink_window(second.link)
        assert connection.sessions()[0].pane == first.pane
        assert connection.window(first.window).pane == first.pane
        assert connection.window(first.window).name == "python:shared.1"
        connection.send_pane_input(first.pane, b"shared-pane\n")
        assert b"shared-pane" in connection.wait_pane_output(first.pane, b"shared-pane", 500)
        connection.unlink_window(link.id)
        assert connection.sessions() == []
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.window(first.window)
        assert failure.value.code == "not_found"


def test_split_pane_exposes_layout_and_both_programs(server: str) -> None:
    """A typed split exposes exact geometry while both owned PTY programs remain usable."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("split", "/bin/cat")
        split = connection.split_pane(
            session.pane,
            tmux_cxx.PaneSplitOrientation.LEFT_RIGHT,
            "printf 'right-ready\\n'; exec cat",
        )
        panes = connection.panes(session.window)
        assert [(pane.id, pane.left, pane.columns, pane.active) for pane in panes] == [
            (session.pane, 0, 40, False),
            (split.id, 41, 39, True),
        ]
        assert all((pane.top, pane.rows, pane.window) == (0, 24, session.window) for pane in panes)
        assert b"right-ready" in connection.wait_pane_output(split.id, b"right-ready", 500)
        resized = connection.resize_pane(split.id, tmux_cxx.PaneResizeDirection.LEFT, 5)
        assert (resized.columns, resized.left, resized.active) == (44, 36, True)
        assert connection.resize_pane(split.id, tmux_cxx.PaneResizeDirection.LEFT, 0).revision == (
            resized.revision
        )
        with pytest.raises(TypeError):
            connection.resize_pane(split.id, tmux_cxx.PaneResizeDirection.LEFT, -1)
        assert connection.window(session.window).revision == resized.revision
        selected = connection.select_pane(session.pane)
        assert (selected.id, selected.active) == (session.pane, True)
        assert [pane.active for pane in connection.panes(session.window)] == [True, False]
        connection.send_pane_input(session.pane, b"left-program\\n")
        assert b"left-program" in connection.wait_pane_output(session.pane, b"left-program", 500)


def test_select_layout_preserves_panes_and_resets_manual_geometry(server: str) -> None:
    """Layouts preserve pane order, insertion, programs, selection, and no-op revisions."""
    with tmux_cxx.ServerConnection(server) as connection:
        assert callable(getattr(connection, "select_layout", None)), (
            "Python must expose SelectLayout"
        )
        session = connection.create_session("layout-presets", "/bin/cat")
        second = connection.split_pane(session.pane, tmux_cxx.PaneSplitOrientation.LEFT_RIGHT)
        third = connection.split_pane(second.id, tmux_cxx.PaneSplitOrientation.TOP_BOTTOM)
        mirrored = connection.select_layout(
            session.window, tmux_cxx.WindowLayoutPreset.MAIN_VERTICAL_MIRRORED
        )
        panes = connection.panes(session.window)
        assert [pane.id for pane in panes] == [session.pane, second.id, third.id]
        assert panes[0].left > panes[1].left == panes[2].left
        assert mirrored.pane == third.id
        inserted = connection.split_pane(session.pane, tmux_cxx.PaneSplitOrientation.TOP_BOTTOM)
        assert [pane.id for pane in connection.panes(session.window)] == [
            session.pane,
            inserted.id,
            second.id,
            third.id,
        ]
        arranged = connection.select_layout(
            session.window, tmux_cxx.WindowLayoutPreset.EVEN_HORIZONTAL
        )
        assert arranged.pane == inserted.id
        connection.resize_pane(session.pane, tmux_cxx.PaneResizeDirection.RIGHT, 3)
        restored = connection.select_layout(
            session.window, tmux_cxx.WindowLayoutPreset.EVEN_HORIZONTAL
        )
        assert [pane.columns for pane in connection.panes(session.window)] == [20, 19, 19, 19]
        assert restored.revision > arranged.revision
        assert (
            connection.select_layout(
                session.window, tmux_cxx.WindowLayoutPreset.EVEN_HORIZONTAL
            ).revision
            == restored.revision
        )
        with pytest.raises(TypeError):
            connection.select_layout(-1, tmux_cxx.WindowLayoutPreset.TILED)
        with pytest.raises(ValueError):
            tmux_cxx.WindowLayoutPreset(999999)
        assert connection.window(session.window).revision == restored.revision
        connection.send_pane_input(second.id, b"printf 'layout-%s\\n' 'python-pty'\n")
        assert b"layout-python-pty" in connection.wait_pane_output(second.id, b"layout-python-pty")
        connection.kill_session(session.id)
        with pytest.raises(tmux_cxx.TmuxError) as stale:
            connection.select_layout(session.window, tmux_cxx.WindowLayoutPreset.TILED)
        assert stale.value.code == "not_found"
    with pytest.raises(tmux_cxx.TmuxError) as closed:
        connection.select_layout(session.window, tmux_cxx.WindowLayoutPreset.TILED)
    assert closed.value.code == "closed"


def test_named_paste_buffers_preserve_bytes_and_drive_panes(server: str) -> None:
    """Both storage and pane delivery use the compiled paste-buffer operations."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("paste", "/bin/sh")
        binary = b"binary\x00\xff"
        connection.set_paste_buffer("binary", binary)
        assert connection.read_paste_buffer("binary") == binary

        command = b"printf 'paste-%s\\n' 'token'\n"
        connection.set_paste_buffer("command", command)
        connection.paste_buffer_into_pane("command", session.pane)
        assert b"paste-token" in connection.wait_pane_output(session.pane, b"paste-token", 500)
        assert connection.read_paste_buffer("command") == command

        connection.delete_paste_buffer("binary")
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.read_paste_buffer("binary")
        assert failure.value.code == "not_found"
        assert failure.value.operation == "paste_buffer.read"


def test_automatic_paste_buffers_skip_named_buffers(server: str) -> None:
    """Automatic creation, metadata, and default paste share the compiled server store."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("automatic-paste", "/bin/sh")
        connection.paste_top_buffer(session.pane)
        assert connection.add_paste_buffer(b"") is None
        assert connection.paste_buffers() == []
        command = b"printf 'automatic-%s\\n' 'python-paste'\n"
        created = connection.add_paste_buffer(command)
        assert isinstance(created, tmux_cxx.PasteBufferSnapshot)
        assert created.name == "buffer0"
        assert created.byte_count == len(command)
        assert created.automatic and created.revision > 0
        connection.set_paste_buffer("named", b"not a shell command")
        metadata = connection.paste_buffers()
        assert [buffer.name for buffer in metadata] == ["named", "buffer0"]
        assert [buffer.automatic for buffer in metadata] == [False, True]
        assert metadata[1].revision == created.revision
        connection.paste_top_buffer(session.pane)
        assert b"automatic-python-paste" in connection.wait_pane_output(
            session.pane, b"automatic-python-paste", 500
        )
        binary = connection.add_paste_buffer(b"binary\x00\xff")
        assert binary is not None
        assert connection.read_paste_buffer(binary.name) == b"binary\x00\xff"
        connection.kill_session(session.id)
        with pytest.raises(tmux_cxx.TmuxError) as stale:
            connection.paste_top_buffer(session.pane)
        assert stale.value.code == "not_found"
        assert stale.value.operation == "paste_buffer.paste_top"
    with pytest.raises(tmux_cxx.TmuxError) as closed:
        connection.add_paste_buffer(b"closed")
    assert closed.value.code == "closed"


def test_removing_hup_ignoring_program_keeps_server_responsive(server: str) -> None:
    """Pane removal admits requests while the server terminates and reaps a HUP-ignoring child."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session(
            "stubborn",
            "trap '' HUP; printf '\\nowned-child=%s:ready\\n' \"$$\"; while :; do :; done",
        )
        output = connection.wait_pane_output(session.pane, b":ready", 500)
        match = re.search(rb"owned-child=(\d+):ready", output)
        assert match is not None
        child = int(match[1])
        descriptor = os.pidfd_open(child)
        try:
            start = time.perf_counter()
            connection.kill_session(session.id)
            assert time.perf_counter() - start < 0.15
            following = connection.create_session("following", "/bin/cat")
            assert [entry.id for entry in connection.sessions()] == [following.id]
            with selectors.DefaultSelector() as exited:
                exited.register(descriptor, selectors.EVENT_READ)
                assert exited.select(timeout=0.7), "owned child outlived the shutdown bound"
            connection.sessions()
            assert not Path(f"/proc/{child}").exists(), "owned direct child was not reaped"
        finally:
            with suppress(ProcessLookupError):
                signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            os.close(descriptor)


def test_first_error_pins_server_identity(server_process: OwnedServer) -> None:
    """A connection first observing an error must reject mutations after the daemon restarts."""
    connection = tmux_cxx.ServerConnection(str(server_process.socket))
    with pytest.raises(tmux_cxx.TmuxError) as first:
        connection.rename_session(99999, "missing")
    assert first.value.server_instance
    server_process.restart()
    with tmux_cxx.ServerConnection(str(server_process.socket)) as current:
        session = current.create_session("current", "/bin/cat")
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.rename_session(session.id, "wrong-server")
        assert failure.value.code == "wrong_server"
        assert current.sessions()[0].name == "current"


def test_daemon_death_stops_owned_child(server_process: OwnedServer) -> None:
    """A killed fixture daemon must stop its direct child without relying on normal shutdown."""
    with tmux_cxx.ServerConnection(str(server_process.socket)) as connection:
        session = connection.create_session(
            "owned", "trap '' HUP; printf '\\nowned-child=%s:ready\\n' \"$$\"; while :; do :; done"
        )
        match = re.search(
            rb"owned-child=(\d+):ready", connection.wait_pane_output(session.pane, b":ready", 500)
        )
        assert match is not None
        descriptor = os.pidfd_open(int(match[1]))
        try:
            server_process.expected_exit = -signal.SIGKILL
            server_process.process.kill()
            with selectors.DefaultSelector() as exited:
                exited.register(descriptor, selectors.EVENT_READ)
                assert exited.select(timeout=0.2), "direct child survived its owned daemon"
        finally:
            with suppress(ProcessLookupError):
                signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            os.close(descriptor)
