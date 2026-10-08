#include "connection/native_server_channel.hpp"
#include <tmux_cxx/session/operations/kill_session.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::kill_session(SessionId session) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(session.value);
    auto reply = channel_->invoke_operation(KillSession::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(KillSession::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess session removal reply bytes"));
    }
    return {};
}
} // namespace tmux_cxx
