"""Session observations and streams exercised through the built Python extension."""

import asyncio
import re
from concurrent.futures import ThreadPoolExecutor

import pytest
import tmux_cxx


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


# Journal recovery requires 4,097 real mutations and belongs in the outer loop.
@pytest.mark.slow
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
