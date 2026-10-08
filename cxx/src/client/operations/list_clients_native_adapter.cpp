#include "connection/native_server_channel.hpp"
#include <tmux_cxx/client/operations/list_clients.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::vector<ClientSnapshot>> ServerConnection::clients() {
    auto reply = channel_->invoke_operation(ListClients::description, {});
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto count = reader.u32();
        if (count > 128) {
            throw std::invalid_argument("excess client count");
        }
        std::vector<ClientSnapshot> clients;
        clients.reserve(count);
        std::uint64_t previous = 0;
        for (std::uint32_t index = 0; index < count; ++index) {
            auto client = reader.client();
            if (client.id.value <= previous) {
                throw std::invalid_argument("unordered or duplicate client identity");
            }
            previous = client.id.value;
            clients.push_back(std::move(client));
        }
        reader.require_end();
        return clients;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ListClients::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
