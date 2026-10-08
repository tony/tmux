#pragma once

#include "protocol/native/framing.hpp"

#include <tmux_cxx/session/session_event.hpp>

namespace tmux_cxx::protocol::native {
/** @brief Append one bounded committed-event batch to a native payload. */
void write_session_event_batch(NativePayloadWriter &writer, const SessionEventBatch &batch);
/** @brief Decode a bounded event batch that continues the requested owner-qualified cursor. */
SessionEventBatch read_session_event_batch(NativePayloadReader &reader,
                                           const SessionEventCursor &requested_cursor);
} // namespace tmux_cxx::protocol::native
