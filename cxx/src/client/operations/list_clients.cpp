#include <tmux_cxx/client/operations/list_clients.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
std::vector<ClientSnapshot> Server::clients() const {
    std::vector<ClientSnapshot> snapshots;
    snapshots.reserve(state_->clients.size());
    for (const auto &[client_id, client] : state_->clients) {
        (void)client_id;
        snapshots.push_back(state_->client_snapshot(*client));
    }
    return snapshots;
}

ListClients::Request ListClients::read(protocol::native::NativePayloadReader &) { return {}; }

Result<ListClients::Response> ListClients::execute(Server &server, Request) {
    return server.clients();
}

protocol::native::OperationReply ListClients::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.size()));
    for (const auto &client : response) {
        reply.client(client);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
