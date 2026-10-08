"""Asyncio scheduling for one C++-owned session event cursor."""

from __future__ import annotations

import asyncio
import threading
from collections.abc import AsyncIterator
from contextlib import suppress
from typing import Self

from ._native import (
    SessionEventGap,
    SessionEventRecord,
    SessionEventStream,
    SessionObservation,
    TmuxError,
)

type SessionEventStreamItem = SessionEventRecord | SessionEventGap


class AsyncSessionEventStream:
    """Adapt one native session cursor to Python's event loop and async iteration."""

    __slots__ = ("_closed", "_read_admission", "_stream")

    def __init__(self, stream: SessionEventStream) -> None:
        """Adopt one native stream without duplicating its observer-owned cursor."""
        self._stream = stream
        self._read_admission = threading.Lock()
        self._closed = False

    @property
    def initial(self) -> SessionObservation:
        """Return the atomic baseline established before this stream became visible."""
        return self._stream.initial

    async def next(self, timeout_ms: int = 500) -> SessionEventStreamItem | None:
        """Await one event or recovery gap while leaving the event loop responsive."""
        if (
            isinstance(timeout_ms, bool)
            or not isinstance(timeout_ms, int)
            or not 1 <= timeout_ms <= 750
        ):
            raise ValueError("timeout_ms must be an integer from 1 through 750")
        if not self._read_admission.acquire(blocking=False):
            raise RuntimeError("SessionEventStream already has a pending read")
        pending = asyncio.create_task(asyncio.to_thread(self._stream.next, timeout_ms))
        try:
            return await asyncio.shield(pending)
        except asyncio.CancelledError:
            self.close()
            with suppress(TmuxError, asyncio.CancelledError):
                await asyncio.shield(pending)
            raise
        finally:
            self._read_admission.release()

    async def events(self, timeout_ms: int = 500) -> AsyncIterator[SessionEventStreamItem]:
        """Yield committed events and explicit recovery gaps until this stream closes."""
        while not self._closed:
            try:
                item = await self.next(timeout_ms)
            except TmuxError as error:
                if self._closed and error.code in {"cancelled", "closed"}:
                    return
                raise
            if item is not None:
                yield item

    def __aiter__(self) -> AsyncIterator[SessionEventStreamItem]:
        """Create a pull-driven iterator over this stream's owned cursor."""
        return self.events()

    def close(self) -> None:
        """Interrupt an admitted wait and reject later reads without changing sessions."""
        self._closed = True
        self._stream.close()

    async def aclose(self) -> None:
        """Close this stream through the asynchronous resource protocol."""
        self.close()

    async def __aenter__(self) -> Self:
        """Borrow this stream without transferring its observer-owned cursor."""
        return self

    async def __aexit__(
        self, exc_type: object | None, exc_value: object | None, traceback: object | None
    ) -> None:
        """Close this stream without suppressing the context's exception."""
        self.close()
