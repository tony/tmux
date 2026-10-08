from tmux_cxx import (
    AsyncSessionEventStream,
    ServerConnection,
    SessionEventGap,
    SessionEventRecord,
    SessionEventStream,
    SessionEventStreamItem,
    SessionObservation,
    SessionSnapshot,
    StockConnectionCompatibility,
)


async def consume_session_events(client: ServerConnection) -> None:
    """Type-check Python's asynchronous session stream and disposal surface."""
    stream: AsyncSessionEventStream = await client.subscribe_sessions_async()
    event: SessionEventStreamItem | None = await stream.next()
    async for observed in stream.events():
        item: SessionEventStreamItem = observed
        del item
        break
    await stream.aclose()
    del event


def consumer(client: ServerConnection) -> SessionSnapshot:
    """Exercise typed snapshots, raw bytes, identities, and mutations without coercion."""
    created: SessionSnapshot = client.create_session("typed", "/bin/sh")
    pane: int = created.pane
    client.send_pane_input(pane, b"printf ok\\n\n")
    output: bytes = client.read_pane_output(pane)
    assert output is not None
    sessions: list[SessionSnapshot] = client.sessions()
    stream: SessionEventStream = client.subscribe_sessions()
    baseline: SessionObservation = stream.initial
    event: SessionEventRecord | SessionEventGap | None = stream.next()
    compatibility: StockConnectionCompatibility = client.stock_client_compatibility(1)
    client.rename_session(created.id, "next")
    assert compatibility.profile
    assert baseline.next_event.server_instance
    if isinstance(event, SessionEventRecord):
        sequence: int = event.sequence
        assert sequence >= 0
    elif isinstance(event, SessionEventGap):
        missed: int = event.missed_from.next_sequence
        assert missed >= 0
    stream.close()
    return sessions[0]
