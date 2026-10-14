"""Exercise the installed Python extension without build-tree import fallbacks."""

import sys
from pathlib import Path

import tmux_cxx
import tmux_cxx._native as native


def installed_provenance(prefix: Path) -> None:
    """Require both the Python facade and native extension beneath this consumer's install."""
    assert Path(sys.prefix).resolve() == prefix.resolve(), "unexpected Python consumer environment"
    assert tmux_cxx.__file__ is not None
    assert native.__file__ is not None
    for module in (tmux_cxx, native):
        assert module.__file__ is not None
        assert Path(module.__file__).resolve().is_relative_to(prefix.resolve()), (
            "Python imported outside the installed consumer"
        )


def create_session(socket: str, record: Path) -> None:
    """Create three persistent shells, arrange their shared window, and close local admission."""
    with tmux_cxx.ServerConnection(socket) as connection:
        defaults = connection.window_options(None)
        assert defaults.window is None and defaults.mode_keys == tmux_cxx.WindowModeKeys.EMACS
        assert defaults.revision == 0
        connection.set_window_option(None, "mode-keys", "vi")
        session = connection.create_session("installed", "/bin/sh")
        inherited = connection.window_options(session.window)
        assert inherited.mode_keys == tmux_cxx.WindowModeKeys.VI
        assert (
            inherited.window == session.window
            and inherited.server_instance == defaults.server_instance
        )
        connection.set_window_option(session.window, "mode-keys", "emacs")
        connection.rename_session(session.id, "installed-python")
        second = connection.split_pane(session.pane, tmux_cxx.PaneSplitOrientation.LEFT_RIGHT)
        third = connection.split_pane(second.id, tmux_cxx.PaneSplitOrientation.TOP_BOTTOM)
        connection.select_pane(session.pane)
        arranged = connection.select_layout(
            session.window, tmux_cxx.WindowLayoutPreset.MAIN_VERTICAL_MIRRORED
        )
        assert arranged.pane == session.pane
        panes = connection.panes(session.window)
        assert [pane.id for pane in panes] == [session.pane, second.id, third.id]
        assert [(pane.top, pane.left, pane.rows, pane.columns) for pane in panes] == [
            (0, 2, 24, 78),
            (0, 0, 12, 1),
            (13, 0, 11, 1),
        ]
        owner = connection.observe_sessions().next_event.server_instance
        automatic = connection.add_paste_buffer(b"installed-python-buffer")
        assert automatic is not None and automatic.automatic
        connection.set_paste_buffer("installed-named", b"named buffers are skipped")
        assert [buffer.automatic for buffer in connection.paste_buffers()] == [False, True]
        connection.send_pane_input(
            session.pane, b"printf '\\033[2J\\033[Hinstalled-copy-seed\\n'\n"
        )
        connection.wait_pane_output(session.pane, b"\x1b[Hinstalled-copy-seed\r\n")
        connection.enter_copy_mode(session.pane)
        connection.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.HISTORY_START)
        connection.move_copy_cursor_to(session.pane, tmux_cxx.CopyCursorDestination.LINE_START)
        connection.begin_copy_selection(session.pane)
        connection.move_copy_cursor(session.pane, tmux_cxx.CopyCursorDirection.RIGHT, 2)
        copied = connection.copy_mode(session.pane)
        assert copied.server_instance == owner and copied.mode is not None
        assert copied.mode.lines[0].startswith("installed-copy-seed")
        assert copied.mode.selection is not None and copied.mode.selection.end.column == 2
        record.write_text(f"{session.id} {session.pane} {owner}\n")
    try:
        connection.sessions()
    except tmux_cxx.TmuxError as error:
        assert error.code == "closed"
    else:
        raise AssertionError("closed installed Python connection admitted a request")


def observe_session(socket: str, record: Path) -> None:
    """Verify Node's mutation, actual shell output, and persistence after both clients exit."""
    session_text, pane_text, owner = record.read_text().split()
    session_id, pane_id = int(session_text), int(pane_text)
    with tmux_cxx.ServerConnection(socket) as observer:
        observation = observer.observe_sessions()
        assert observation.next_event.server_instance == owner
        assert len(observation.sessions) == 1
        assert observation.sessions[0].id == session_id
        assert observation.sessions[0].name == "installed-node"
        assert observation.sessions[0].pane == pane_id
        window = observation.sessions[0].window
        assert observer.window_options(None).mode_keys == tmux_cxx.WindowModeKeys.VI
        options = observer.window_options(window)
        assert options.window == window and options.server_instance == owner
        assert options.mode_keys == tmux_cxx.WindowModeKeys.EMACS and options.revision > 0
        panes = observer.panes(window)
        assert len(panes) == 3
        assert panes[0].id == pane_id
        assert [pane.active for pane in panes] == [True, False, False]
        assert [(pane.top, pane.left, pane.rows, pane.columns) for pane in panes] == [
            (0, 0, 11, 39),
            (0, 40, 11, 40),
            (12, 0, 12, 80),
        ]
        output = observer.read_pane_output(pane_id)
        assert b"installed-node-current-pty" in output
        assert b"installed-node-lts-pty" in output
        assert b"installed-buffer-node-current-pty" in output
        assert b"installed-buffer-node-lts-pty" in output
        buffers = observer.paste_buffers()
        assert len(buffers) == 6
        assert buffers[0].automatic and buffers[0].revision > 0
        assert observer.read_paste_buffer(buffers[0].name).startswith(b"printf 'installed-buffer-")
        copied = observer.copy_mode(pane_id)
        assert (
            copied.server_instance == owner and copied.pane == pane_id and copied.mode is not None
        )
        assert copied.mode.mode_keys == copied.mode.highlight_keys == tmux_cxx.WindowModeKeys.EMACS
        assert copied.mode.lines[0].startswith("installed-copy-seed")
        assert copied.mode.selection is not None and copied.mode.selection.end.column == 2
        assert copied.mode.cursor.column == 2 and copied.display_revision > 0
        observer.send_pane_input(pane_id, b"printf 'python-%s\\n' 'installed-pty'\n")
        assert b"python-installed-pty" in observer.wait_pane_output(
            pane_id, b"python-installed-pty", 500
        )
        retained = observer.copy_mode(pane_id)
        assert retained.mode is not None
        assert retained.mode.source_revision == copied.mode.source_revision
        assert retained.mode.lines == copied.mode.lines
        assert retained.display_revision == copied.display_revision
        assert observer.read_pane_text(pane_id).revision > copied.mode.source_revision
        observer.cancel_copy_mode(pane_id)
        assert observer.copy_mode(pane_id).mode is None


def main() -> None:
    """Validate installed provenance and run the requested shared-daemon consumer phase."""
    phase, socket, prefix, record = sys.argv[1:]
    installed_provenance(Path(prefix))
    if phase == "create":
        create_session(socket, Path(record))
    elif phase == "observe":
        observe_session(socket, Path(record))
    elif phase != "probe":
        raise ValueError(f"unknown installed consumer phase: {phase}")


if __name__ == "__main__":
    main()
