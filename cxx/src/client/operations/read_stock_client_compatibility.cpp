#include <tmux_cxx/client/operations/read_stock_client_compatibility.hpp>

#include "client/client.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<protocol::tmux::StockConnectionCompatibility>
Server::stock_client_compatibility(ClientId client) const {
    const auto client_entry = state_->clients.find(client);
    if (client_entry == state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    if (!client_entry->second->stock_compatibility) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "client identification is incomplete"});
    }
    return *client_entry->second->stock_compatibility;
}

ReadStockClientCompatibility::Request
ReadStockClientCompatibility::read(protocol::native::NativePayloadReader &reader) {
    return {ClientId{reader.u64()}};
}

Result<ReadStockClientCompatibility::Response>
ReadStockClientCompatibility::execute(Server &server, Request request) {
    return server.stock_client_compatibility(request.client);
}

protocol::native::OperationReply ReadStockClientCompatibility::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.stock_compatibility(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
