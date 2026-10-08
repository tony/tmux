#include "connection/native_server_channel.hpp"
#include "protocol/native/session_event_codec.hpp"
#include <tmux_cxx/session/operations/read_session_events.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<SessionEventBatch> ServerConnection::read_session_events(SessionEventCursor cursor) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(cursor.server_instance);
    arguments.u64(cursor.next_sequence);
    auto reply = channel_->invoke_operation(ReadSessionEvents::description, std::move(arguments));
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
        return std::unexpected(channel_->operation_error(ReadSessionEvents::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        ReadSessionEvents::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
