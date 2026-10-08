"""Verify that C++, Python, Node, and an unmodified stock client share one server."""

import os
import subprocess
from pathlib import Path

import pytest
import tmux_cxx

from tests.conftest import fixture_environment

ROOT = Path(__file__).resolve().parents[2]


def test_all_clients_share_persistent_state(server: str) -> None:
    """Cross all real adapters, exercise PTY bytes and a failure, then verify persistence."""
    environment = fixture_environment()
    created = subprocess.check_output(
        [os.environ["TMUX_CXX_DAEMON"], "--socket", server, "create", "native", "/bin/sh"],
        text=True,
        env=environment,
        timeout=0.9,
    )
    session_id, pane_id = map(int, created.split())
    with tmux_cxx.ServerConnection(server) as connection:
        connection.rename_session(session_id, "python")
    subprocess.run(
        [
            str(ROOT / "_build/tools/node_current/bin/node"),
            str(ROOT / "bindings/node/tests/shared_server.mts"),
            server,
        ],
        env=fixture_environment(preload=True),
        check=True,
        timeout=0.9,
    )
    stock = subprocess.check_output(
        [
            os.environ["TMUX_CXX_STOCK_CLIENT"],
            "-N",
            "-f",
            "/dev/null",
            "-S",
            server,
            "list-sessions",
            "-F",
            "#{session_name}:#{session_id}:#{pane_id}",
        ],
        text=True,
        env=environment,
        timeout=0.9,
    )
    assert stock == f"node:${session_id}:%{pane_id}\n"
    with tmux_cxx.ServerConnection(server) as connection:
        connection.send_pane_input(pane_id, b"printf 'shared-%s\\n' 'pty-token'\n")
        assert b"shared-pty-token" in connection.wait_pane_output(pane_id, b"shared-pty-token", 500)
        with pytest.raises(tmux_cxx.TmuxError) as failure:
            connection.rename_session(99999, "missing")
        assert failure.value.code == "not_found"
    with tmux_cxx.ServerConnection(server) as observer:
        assert observer.sessions()[0].id == session_id
