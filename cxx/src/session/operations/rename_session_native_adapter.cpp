#include "connection/native_server_channel.hpp"
#include <tmux_cxx/session/operations/rename_session.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::rename_session(SessionId session, std::string name) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(session.value);
    arguments.string(name);
    auto reply = channel_->invoke_operation(RenameSession::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(RenameSession::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess session rename reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        RenameSession::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
