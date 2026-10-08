"""Python scheduling facade for the C++ server connection."""

from __future__ import annotations

import asyncio
from typing import Self

from ._native import ServerConnection as _NativeServerConnection
from ._native import SessionEventStream
from .session_event_stream import AsyncSessionEventStream


def _close_cancelled_subscription(subscription: asyncio.Task[SessionEventStream]) -> None:
    """Close a stream created after its subscription task lost its caller."""
    try:
        stream = subscription.result()
    except asyncio.CancelledError:
        return
    except Exception as error:
        subscription.get_loop().call_exception_handler(
            {
                "message": "session subscription failed after caller cancellation",
                "exception": error,
                "task": subscription,
            }
        )
        return
    stream.close()


class ServerConnection(_NativeServerConnection):
    """Control one persistent server through synchronous and asyncio operations."""

    async def subscribe_sessions_async(self) -> AsyncSessionEventStream:
        """Establish one session baseline without blocking the caller's event loop."""
        subscribe = super().subscribe_sessions
        pending = asyncio.create_task(asyncio.to_thread(subscribe))
        try:
            return AsyncSessionEventStream(await asyncio.shield(pending))
        except asyncio.CancelledError:
            pending.add_done_callback(_close_cancelled_subscription)
            raise

    def __enter__(self) -> Self:
        """Borrow this connection without transferring server ownership."""
        return self

    async def __aenter__(self) -> Self:
        """Borrow this connection without blocking or transferring server ownership."""
        return self

    async def __aexit__(
        self, exc_type: object | None, exc_value: object | None, traceback: object | None
    ) -> None:
        """Close admission without suppressing the context's exception."""
        self.close()
