#include "connection/native_server_channel.hpp"
#include <tmux_cxx/client/operations/detach_client.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<ClientSnapshot> ServerConnection::detach_client(ClientId client) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(client.value);
    auto reply = channel_->invoke_operation(DetachClient::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto detached = reader.client();
        reader.require_end();
        if (detached.id != client || detached.session) {
            throw std::invalid_argument("client detachment reply has different ownership");
        }
        return detached;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(DetachClient::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
