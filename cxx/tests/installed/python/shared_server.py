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
    """Create and rename a persistent shell, then close admission without deleting it."""
    with tmux_cxx.ServerConnection(socket) as connection:
        session = connection.create_session("installed", "/bin/sh")
        connection.rename_session(session.id, "installed-python")
        owner = connection.observe_sessions().next_event.server_instance
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
        output = observer.read_pane_output(pane_id)
        assert b"installed-node-current-pty" in output
        assert b"installed-node-lts-pty" in output
        observer.send_pane_input(pane_id, b"printf 'python-%s\\n' 'installed-pty'\n")
        assert b"python-installed-pty" in observer.wait_pane_output(
            pane_id, b"python-installed-pty", 500
        )


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
