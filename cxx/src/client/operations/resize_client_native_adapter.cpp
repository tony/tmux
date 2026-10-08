#include "connection/native_server_channel.hpp"
#include <tmux_cxx/client/operations/resize_client.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<ClientSnapshot> ServerConnection::resize_client(ClientId client) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(client.value);
    auto reply = channel_->invoke_operation(ResizeClient::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto resized = reader.client();
        reader.require_end();
        if (resized.id != client || !resized.session) {
            throw std::invalid_argument("client resize reply has different ownership");
        }
        return resized;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ResizeClient::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
