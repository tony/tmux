#include "connection/native_server_channel.hpp"
#include <tmux_cxx/session/operations/list_sessions.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::vector<SessionSnapshot>> ServerConnection::sessions() {
    auto reply = channel_->invoke_operation(ListSessions::description, {});
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto count = reader.u32();
        if (count > 65536) {
            throw std::invalid_argument("excess session count");
        }
        std::vector<SessionSnapshot> sessions;
        for (std::uint32_t i = 0; i < count; ++i) {
            sessions.push_back(reader.session());
        }
        if (!reader.finished()) {
            throw std::invalid_argument("excess sessions reply bytes");
        }
        return sessions;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ListSessions::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
