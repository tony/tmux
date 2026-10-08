#include "connection/native_server_channel.hpp"
#include <tmux_cxx/client/operations/attach_client.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<ClientSnapshot> ServerConnection::attach_client(ClientId client, SessionId session) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(client.value);
    arguments.u64(session.value);
    auto reply = channel_->invoke_operation(AttachClient::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto attached = reader.client();
        reader.require_end();
        if (attached.id != client || attached.session != session) {
            throw std::invalid_argument("client attachment reply has different ownership");
        }
        return attached;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(AttachClient::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
