"""Exercise registered stock commands against the C++ daemon and owned references."""

import os
import subprocess
from pathlib import Path

import pytest
import tmux_cxx

from tests.conftest import stock_fixture_environment


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_registered_stock_command_subset(server: str, fixture: str) -> None:
    """Unmodified legacy and modern clients mutate one C++ session through registered commands."""
    environment = stock_fixture_environment(os.environ[fixture])
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
def test_stock_window_options_match_owned_reference(server: str, fixture: str) -> None:
    """Both stock releases assign inherited and shared window options against owned references."""
    binary = os.environ[fixture]
    environment = stock_fixture_environment(binary)
    reference_socket = Path(server).with_name("options-reference")
    reference = [binary, "-f", "/dev/null", "-S", str(reference_socket)]
    client = [binary, "-N", "-f", "/dev/null", "-S", server]
    assert not reference_socket.exists()

    def run(endpoint: list[str], *arguments: str) -> bytes:
        """Run one bounded command on the selected owned endpoint."""
        return subprocess.run(
            [*endpoint, *arguments],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        ).stdout

    with tmux_cxx.ServerConnection(server) as observer:
        source = observer.create_session("options-source", "/bin/cat")
        other = observer.create_session("options-other", "/bin/cat")
        observer.link_window(source.window, other.id, 3)
        server_instance = observer.window_options(None).server_instance
        try:
            run(reference, "new-session", "-d", "-s", "options-source", "/bin/cat")
            run(reference, "new-session", "-d", "-s", "options-other", "/bin/cat")
            run(reference, "link-window", "-s", "options-source:0", "-t", "options-other:3")
            for arguments in (
                ("set-option", "-g", "mode-keys", "vi"),
                ("setw", "-t", "options-other:3", "mode-keys", "emacs"),
                ("set", "-gw", "mode-keys", "emacs"),
                ("set-option", "-w", "-t", "options-source:0", "mode-keys", "vi"),
                ("setw", "-t", "options-source:0", "--", "mode-keys", "emacs"),
            ):
                run(client, *arguments)
                run(reference, *arguments)
                for window, target in (
                    (source.window, "options-source:0"),
                    (other.window, "options-other:0"),
                ):
                    actual = observer.window_options(window)
                    expected = run(
                        reference, "show-options", "-w", "-A", "-v", "-t", target, "mode-keys"
                    )
                    assert actual.server_instance == server_instance
                    assert actual.window == window
                    assert actual.mode_keys.name.lower() == expected.decode().strip()
                assert (
                    observer.window_options(None).mode_keys.name.lower()
                    == run(reference, "show-window-options", "-g", "-v", "mode-keys")
                    .decode()
                    .strip()
                )

            before = observer.window_options(source.window)
            for arguments in (
                ("set-option", "-gw", "prefix", "C-x"),
                ("set-window-option", "-t", "options-source:0", "mode-keys", "unknown"),
                ("set-window-option", "-t", "options-source:999", "mode-keys", "emacs"),
                ("set-option", "-u", "-g", "mode-keys"),
            ):
                rejected = subprocess.run(
                    [*client, *arguments], env=environment, capture_output=True, timeout=0.9
                )
                assert rejected.returncode != 0
                after = observer.window_options(source.window)
                assert after.mode_keys == before.mode_keys
                assert after.revision == before.revision
        finally:
            subprocess.run(
                [*reference, "kill-server"],
                env=environment,
                check=False,
                capture_output=True,
                timeout=0.9,
            )
            reference_socket.unlink(missing_ok=True)


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_automatic_buffers_and_default_paste(server: str, fixture: str) -> None:
    """Unmodified clients create automatic buffers and paste past newer named entries."""
    binary = os.environ[fixture]
    base = [binary, "-N", "-f", "/dev/null", "-S", server]
    environment = stock_fixture_environment(binary)
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("stock-default-paste", "/bin/sh")
        commands = [
            ["paste-buffer", "-t", f"%{session.pane}"],
            ["set-buffer", ""],
            ["set-buffer", "printf 'automatic-%s\\n' 'stock-paste'\n"],
            ["set-buffer", "-b", "named", "default paste must skip this"],
            ["paste-buffer", "-t", f"%{session.pane}"],
        ]
        for command in commands:
            subprocess.run(
                [*base, *command],
                env=environment,
                check=True,
                capture_output=True,
                timeout=0.9,
            )
        buffers = observer.paste_buffers()
        assert [buffer.name for buffer in buffers] == ["named", "buffer0"]
        assert [buffer.automatic for buffer in buffers] == [False, True]
        assert observer.read_paste_buffer("buffer0") == commands[2][1].encode()
        assert b"automatic-stock-paste" in observer.wait_pane_output(
            session.pane, b"automatic-stock-paste", 500
        )


@pytest.mark.parametrize("fixture", ["TMUX_CXX_STOCK_CLIENT", "TMUX_CXX_MODERN_STOCK_CLIENT"])
def test_stock_window_creation_and_selection(server: str, fixture: str) -> None:
    """Real clients create, rename, and select live shared windows through explicit targets."""
    base = [os.environ[fixture], "-N", "-f", "/dev/null", "-S", server]
    environment = stock_fixture_environment(os.environ[fixture])
    with tmux_cxx.ServerConnection(server) as observer:
        session = observer.create_session("windows", "/bin/cat")
        subprocess.run(
            [*base, "new-window", "-d", "-t", "windows:7", "-n", "shared", "/bin/cat"],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        )
        links = observer.window_links(session.id)
        assert [link.index for link in links] == [0, 7]
        assert observer.sessions()[0].link == session.link
        subprocess.run(
            [*base, "renamew", "-t", "windows:7", "stock:shared.1"],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        )
        assert observer.window(links[1].window).name == "stock:shared.1"
        before_rename_rejection = observer.window(links[1].window)
        rejected_rename = subprocess.run(
            [*base, "rename-window", "-t", "windows:7", "#{pane_current_command}"],
            env=environment,
            capture_output=True,
            timeout=0.9,
        )
        assert rejected_rename.returncode != 0
        assert observer.window(links[1].window).name == before_rename_rejection.name
        assert observer.window(links[1].window).revision == before_rename_rejection.revision
        subprocess.run(
            [*base, "selectw", "-t", "windows:7"],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        )
        selected = observer.sessions()[0]
        assert selected.link == links[1].id
        assert selected.window == links[1].window
        for command, expected_link in (("next", session.link), ("prev", links[1].id)):
            subprocess.run(
                [*base, command, "-t", "windows"],
                env=environment,
                check=True,
                capture_output=True,
                timeout=0.9,
            )
            assert observer.sessions()[0].link == expected_link
        observer.send_pane_input(selected.pane, b"stock-window-pty\n")
        assert b"stock-window-pty" in observer.wait_pane_output(selected.pane, b"stock-window-pty")
        subprocess.run(
            [*base, "neww", "-t", f"${session.id}", "-n", "automatic-index", "/bin/cat"],
            env=environment,
            check=True,
            capture_output=True,
            timeout=0.9,
        )
        links = observer.window_links(session.id)
        assert [link.index for link in links] == [0, 1, 7]
        assert observer.sessions()[0].link == links[1].id
        rejected = subprocess.run(
            [*base, "new-window", "-k", "-t", "windows:1", "-n", "rejected", "/bin/cat"],
            env=environment,
            capture_output=True,
            timeout=0.9,
        )
        assert rejected.returncode != 0
        assert [link.id for link in observer.window_links(session.id)] == [
            link.id for link in links
        ]
        assert observer.window(links[1].window).name == "automatic-index"
