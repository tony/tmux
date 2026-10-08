"""Critical persistent server behavior exercised through the built Python extension."""

import asyncio
import hashlib
import os
import re
import selectors
import signal
import time
from concurrent.futures import ThreadPoolExecutor
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


def test_duplicate_rejection_preserves_state(server: str) -> None:
    """Rejecting a duplicate name preserves the existing session."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("existing", "/bin/sh")
        with pytest.raises(tmux_cxx.TmuxError):
            connection.create_session("existing", "/bin/sh")
        assert [s.id for s in connection.sessions()] == [session.id]


def test_session_observation_carries_its_next_event_cursor(server: str) -> None:
    """A materialized session catalog carries its owner-qualified following event position."""
    with tmux_cxx.ServerConnection(server) as connection:
        session = connection.create_session("observed", "/bin/cat")
        observation = connection.observe_sessions()
        assert [value.id for value in observation.sessions] == [session.id]
        assert re.fullmatch(r"[0-9a-f]{32}", observation.next_event.server_instance)
        assert observation.next_event.next_sequence >= 1
        connection.rename_session(session.id, "renamed-after-observation")
        changes = connection.read_session_events(observation.next_event)
        assert changes.next_cursor.server_instance == observation.next_event.server_instance
        assert changes.next_cursor.next_sequence == observation.next_event.next_sequence + 1
        assert len(changes.records) == 1
        assert changes.records[0].sequence == observation.next_event.next_sequence
        assert isinstance(changes.records[0].event, tmux_cxx.SessionRenamedEvent)
        assert changes.records[0].event.session == session.id
        assert changes.records[0].event.name == "renamed-after-observation"


def test_waiting_for_session_events_allows_another_client_to_mutate(server: str) -> None:
    """A pending metadata observation does not block another client's committed mutation."""
    with (
        tmux_cxx.ServerConnection(server) as observer,
        tmux_cxx.ServerConnection(server) as mutator,
        ThreadPoolExecutor(max_workers=1) as worker,
    ):
        session = mutator.create_session("waiting", "/bin/cat")
        cursor = observer.observe_sessions().next_event
        pending = worker.submit(observer.wait_session_events, cursor, 500)
        mutator.rename_session(session.id, "observed-mutation")
        changes = pending.result(timeout=0.75)
        assert len(changes.records) == 1
        assert isinstance(changes.records[0].event, tmux_cxx.SessionRenamedEvent)
        assert changes.records[0].event.name == "observed-mutation"


def test_session_event_stream_establishes_and_advances_its_baseline(server: str) -> None:
    """A session stream owns an atomic baseline and advances one committed record at a time."""
    with (
        tmux_cxx.ServerConnection(server) as observer,
        tmux_cxx.ServerConnection(server) as mutator,
    ):
        session = mutator.create_session("streamed", "/bin/cat")
        stream = observer.subscribe_sessions()
        assert [value.id for value in stream.initial.sessions] == [session.id]
        mutator.rename_session(session.id, "streamed-rename")
        update = stream.next(500)
        assert isinstance(update, tmux_cxx.SessionEventRecord)
        assert update.sequence == stream.initial.next_event.next_sequence
        assert isinstance(update.event, tmux_cxx.SessionRenamedEvent)
        assert update.event.name == "streamed-rename"
        stream.close()
        with pytest.raises(tmux_cxx.TmuxError) as closed:
            stream.next(500)
        assert closed.value.code == "closed"


def test_session_event_stream_recovers_after_retention_gap(server: str) -> None:
    """A discarded stream prefix yields one replacement baseline before delivery resumes."""
    with (
        tmux_cxx.ServerConnection(server) as observer,
        tmux_cxx.ServerConnection(server) as mutator,
    ):
        session = mutator.create_session("gap-start", "/bin/cat")
        stream = observer.subscribe_sessions()
        for sequence in range(4097):
            mutator.rename_session(session.id, "gap-a" if sequence % 2 == 0 else "gap-b")

        gap = stream.next(500)
        assert isinstance(gap, tmux_cxx.SessionEventGap)
        assert gap.missed_from.next_sequence == stream.initial.next_event.next_sequence
        assert gap.replacement.sessions[0].name == "gap-a"
        mutator.rename_session(session.id, "gap-after-recovery")
        resumed = stream.next(500)
        assert isinstance(resumed, tmux_cxx.SessionEventRecord)
        assert resumed.sequence == gap.replacement.next_event.next_sequence


def test_async_session_event_stream_yields_without_blocking_the_loop(server: str) -> None:
    """Async session delivery leaves the event loop live and closes on task cancellation."""

    async def exercise() -> None:
        """Drive one event, one async iteration, and one cancelled native wait."""
        async with (
            tmux_cxx.ServerConnection(server) as observer,
            tmux_cxx.ServerConnection(server) as mutator,
        ):
            session = await asyncio.to_thread(mutator.create_session, "async-stream", "/bin/cat")
            stream = await observer.subscribe_sessions_async()
            with pytest.raises(ValueError, match="1 through 750"):
                await stream.next(0)
            loop_progress = asyncio.Event()
            pending = asyncio.create_task(stream.next())
            asyncio.get_running_loop().call_soon(loop_progress.set)
            await loop_progress.wait()
            assert not pending.done()
            with pytest.raises(RuntimeError, match="already has a pending read"):
                await stream.next()
            await asyncio.to_thread(mutator.rename_session, session.id, "async-next")
            delivered = await pending
            assert isinstance(delivered, tmux_cxx.SessionEventRecord)
            assert isinstance(delivered.event, tmux_cxx.SessionRenamedEvent)
            assert delivered.event.name == "async-next"

            iterator = stream.events().__aiter__()
            iterated = asyncio.ensure_future(anext(iterator))
            await asyncio.to_thread(mutator.rename_session, session.id, "async-iteration")
            observed = await iterated
            assert isinstance(observed, tmux_cxx.SessionEventRecord)
            assert isinstance(observed.event, tmux_cxx.SessionRenamedEvent)
            assert observed.event.name == "async-iteration"
            await stream.aclose()

            cancellable = await observer.subscribe_sessions_async()
            cancelled = asyncio.create_task(cancellable.next())
            cancellation_admitted = asyncio.Event()
            asyncio.get_running_loop().call_soon(cancellation_admitted.set)
            await cancellation_admitted.wait()
            cancelled.cancel()
            with pytest.raises(asyncio.CancelledError):
                await cancelled
            with pytest.raises(tmux_cxx.TmuxError) as closed:
                await cancellable.next()
            assert closed.value.code == "closed"

    asyncio.run(exercise())


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
        connection.kill_session(first.id)
        connection.unlink_window(second.link)
        assert connection.sessions()[0].pane == first.pane
        assert connection.window(first.window).pane == first.pane
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
        selected = connection.select_pane(session.pane)
        assert (selected.id, selected.active) == (session.pane, True)
        assert [pane.active for pane in connection.panes(session.window)] == [True, False]
        connection.send_pane_input(session.pane, b"left-program\\n")
        assert b"left-program" in connection.wait_pane_output(session.pane, b"left-program", 500)


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
