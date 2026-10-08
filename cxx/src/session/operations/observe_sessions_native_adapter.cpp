#include "connection/native_server_channel.hpp"
#include <tmux_cxx/session/operations/observe_sessions.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<SessionObservation> ServerConnection::observe_sessions() {
    return observe_sessions_until_cancelled({});
}

Result<SessionObservation>
ServerConnection::observe_sessions_until_cancelled(std::stop_token stop_token) {
    auto reply = channel_->invoke_operation(ObserveSessions::description, {}, stop_token);
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto count = reader.u32();
        if (count > 65536) {
            throw std::invalid_argument("excess observed session count");
        }
        SessionObservation observation;
        observation.sessions.reserve(count);
        for (std::uint32_t index = 0; index < count; ++index) {
            observation.sessions.push_back(reader.session());
        }
        observation.next_event = {reader.string(), reader.u64()};
        reader.require_end();
        if (observation.next_event.server_instance != channel_->observed_server_instance()) {
            throw std::invalid_argument("session observation cursor does not match reply owner");
        }
        return observation;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ObserveSessions::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
