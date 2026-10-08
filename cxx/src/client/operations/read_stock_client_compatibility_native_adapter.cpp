#include "connection/native_server_channel.hpp"
#include <tmux_cxx/client/operations/read_stock_client_compatibility.hpp>

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<protocol::tmux::StockConnectionCompatibility>
ServerConnection::stock_client_compatibility(ClientId client) {
    protocol::native::NativePayloadWriter request;
    request.u64(client.value);
    auto reply =
        channel_->invoke_operation(ReadStockClientCompatibility::description, std::move(request));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto compatibility = reader.stock_compatibility();
        reader.require_end();
        return compatibility;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ReadStockClientCompatibility::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
