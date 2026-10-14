"""Exercise stock copy commands through unmodified clients and shared native observation."""

import os
import subprocess

import pytest
import tmux_cxx

from tests.conftest import stock_fixture_environment


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_copy_commands_preserve_frozen_state(server: str, fixture: str) -> None:
    """Stock copy commands navigate frozen cells and reject unsupported syntax before effects."""
    binary = os.environ[fixture]
    environment = stock_fixture_environment(binary)
    base = [binary, "-N", "-f", "/dev/null", "-S", server]

    def run(*arguments: str) -> bytes:
        """Run one supported command through the selected stock client on its owned socket."""
        return subprocess.run(
            [*base, *arguments], env=environment, capture_output=True, check=True, timeout=0.9
        ).stdout

    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("stock-copy", "/bin/cat")
        target = f"%{session.pane}"
        observer.send_pane_input(session.pane, b"abc\n")
        observer.wait_pane_output(session.pane, b"abc\r\n")
        run("copy-mode", "-t", target)
        frozen = observer.copy_mode(session.pane)
        assert frozen.mode is not None and frozen.mode.lines[0].startswith("abc")
        run("send-keys", "-X", "-t", target, "history-top")
        run("send-keys", "-X", "-N", "7", "-t", target, "start-of-line")
        run("send", "-X", "-t", target, "begin-selection")
        run("send-keys", "-X", "-N", "2", "-t", target, "cursor-right")
        selected = observer.copy_mode(session.pane)
        assert selected.mode is not None and selected.mode.selection is not None
        assert selected.mode.selection.anchor.column == 0
        assert selected.mode.selection.end.column == 2
        run("copy-mode", "-t", target)
        assert observer.copy_mode(session.pane).display_revision == selected.display_revision
        run("send-keys", "-X", "-t", target, "copy-selection-no-clear")
        assert observer.read_paste_buffer(observer.paste_buffers()[0].name) == b"ab"
        assert observer.copy_mode(session.pane).mode is not None
        for arguments in (
            ("copy-mode",),
            ("copy-mode", "-u", "-t", target),
            ("copy-mode", "-t", target, "extra"),
            ("send-keys", "-t", target, "cursor-left"),
            ("send-keys", "-X", "cursor-left"),
            ("send-keys", "-X", "-N", "0", "-t", target, "cursor-left"),
            ("send-keys", "-X", "-N", "invalid", "-t", target, "cursor-left"),
            ("send-keys", "-X", "-t", target, "copy-pipe-and-cancel"),
            ("send-keys", "-X", "-t", target, "search-forward", "abc"),
        ):
            rejected = subprocess.run(
                [*base, *arguments], env=environment, capture_output=True, timeout=0.9
            )
            assert rejected.returncode != 0
            after = observer.copy_mode(session.pane)
            assert after.mode is not None and after.mode.selection is not None
            assert after.display_revision == selected.display_revision
            assert after.mode.selection.end.column == 2
        run("send-keys", "-X", "-t", target, "copy-selection-and-cancel")
        assert observer.copy_mode(session.pane).mode is None
        rejected = subprocess.run(
            [*base, "send-keys", "-X", "-t", target, "cancel"],
            env=environment,
            capture_output=True,
            timeout=0.9,
        )
        assert rejected.returncode != 0
        assert observer.copy_mode(session.pane).mode is None
