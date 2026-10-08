#include <tmux_cxx/client/operations/detach_client.hpp>

#include "client/client.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<ClientSnapshot> Server::detach_client(ClientId client) {
    const auto client_entry = state_->clients.find(client);
    if (client_entry == state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    auto &client_state = *client_entry->second;
    if (!client_state.session) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "client is not attached to a session"});
    }
    const auto session = state_->sessions.find(*client_state.session);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    if (client_state.terminal) {
        if (auto restoration = client_state.terminal->restore(); !restoration) {
            return std::unexpected(restoration.error());
        }
    }
    client_state.detached_from = session->second.name;
    client_state.previous_session = client_state.session;
    client_state.session.reset();
    client_state.view.invalidate();
    client_state.revision = ++state_->revision;
    return state_->client_snapshot(client_state);
}

DetachClient::Request DetachClient::read(protocol::native::NativePayloadReader &reader) {
    return {ClientId{reader.u64()}};
}

Result<DetachClient::Response> DetachClient::execute(Server &server, Request request) {
    return server.detach_client(request.client);
}

protocol::native::OperationReply DetachClient::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.client(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
