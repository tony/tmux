#include "connection/native_server_channel.hpp"
#include "protocol/native/session_event_codec.hpp"
#include <tmux_cxx/session/operations/wait_session_events.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<SessionEventBatch> ServerConnection::wait_session_events(SessionEventCursor cursor,
                                                                std::uint32_t timeout_ms) try {
    return wait_session_events_until_cancelled(std::move(cursor), timeout_ms, {});
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        WaitSessionEvents::description, TmuxErrorCode::invalid_argument, error.what()));
}

Result<SessionEventBatch> ServerConnection::wait_session_events_until_cancelled(
    SessionEventCursor cursor, std::uint32_t timeout_ms, std::stop_token stop_token) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(cursor.server_instance);
    arguments.u64(cursor.next_sequence);
    arguments.u32(timeout_ms);
    auto reply = channel_->invoke_operation(WaitSessionEvents::description, std::move(arguments),
                                            stop_token);
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        if (cursor.server_instance != channel_->observed_server_instance()) {
            throw std::invalid_argument("session event cursor does not match reply owner");
        }
        auto batch = protocol::native::read_session_event_batch(reader, cursor);
        reader.require_end();
        return batch;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(WaitSessionEvents::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        WaitSessionEvents::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
