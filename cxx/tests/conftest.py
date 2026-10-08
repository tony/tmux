"""Own C++ daemon processes, sockets, readiness pipes, and bounded cleanup."""

import os
import selectors
import signal
import subprocess
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest


def fixture_environment(*, preload: bool = False) -> dict[str, str]:
    """Pass only fixture paths and runtime settings; preload ASan only for addon hosts."""
    allowed = {
        "PATH",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "TERM",
        "LD_LIBRARY_PATH",
        "ASAN_SYMBOLIZER_PATH",
        "ASAN_OPTIONS",
        "UBSAN_OPTIONS",
        "LSAN_OPTIONS",
    }
    if preload:
        allowed.add("LD_PRELOAD")
    return {k: v for k, v in os.environ.items() if k in allowed or k.startswith("TMUX_CXX_")}


@dataclass
class OwnedServer:
    """Retain exactly one fixture daemon and its expected exit status."""

    socket: Path
    process: subprocess.Popen[bytes]
    expected_exit: int = 0

    def stop(self) -> None:
        """Stop only this fixture's daemon and reject slow or unexpected shutdown."""
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGTERM)
        try:
            code = self.process.wait(timeout=0.85)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=0.15)
            raise AssertionError("owned daemon exceeded its shutdown bound") from None
        assert code == self.expected_exit

    def restart(self) -> None:
        """Replace the owned daemon at the same endpoint after verifying its clean shutdown."""
        self.stop()
        self.process = launch_server(self.socket).process


def launch_server(socket: Path) -> OwnedServer:
    """Start an owned daemon and admit requests only after its readiness byte."""
    read_fd, write_fd = os.pipe()
    try:
        child = subprocess.Popen(
            [
                os.environ["TMUX_CXX_DAEMON"],
                "serve",
                "--socket",
                str(socket),
                "--ready-fd",
                str(write_fd),
            ],
            pass_fds=(write_fd,),
            env=fixture_environment(),
        )
    except BaseException:
        os.close(read_fd)
        raise
    finally:
        os.close(write_fd)
    try:
        with selectors.DefaultSelector() as readiness:
            readiness.register(read_fd, selectors.EVENT_READ)
            assert readiness.select(timeout=0.75), "owned daemon did not signal readiness"
            assert os.read(read_fd, 1) == b"R", "owned daemon exited before readiness"
    except BaseException:
        child.kill()
        child.wait(timeout=0.15)
        raise
    finally:
        os.close(read_fd)
    return OwnedServer(socket, child)


@pytest.fixture
def server_process(tmp_path_factory: pytest.TempPathFactory) -> Iterator[OwnedServer]:
    """Yield a ready daemon with a short owned socket path and verify cleanup."""
    daemon = launch_server(tmp_path_factory.mktemp("server") / "socket")
    try:
        yield daemon
    finally:
        daemon.stop()


@pytest.fixture
def server(server_process: OwnedServer) -> str:
    """Borrow this fixture's socket without owning persistent server entities."""
    return str(server_process.socket)
