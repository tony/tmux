#include "connection/native_server_channel.hpp"
#include <tmux_cxx/session/operations/create_session.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<SessionSnapshot> ServerConnection::create_session(std::string name,
                                                         std::string command) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(name);
    arguments.string(command);
    auto reply = channel_->invoke_operation(CreateSession::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto session = reader.session();
        if (!reader.finished()) {
            throw std::invalid_argument("excess session reply bytes");
        }
        return session;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(CreateSession::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        CreateSession::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
