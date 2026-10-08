#include <tmux_cxx/client/operations/attach_client.hpp>

#include "client/client.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "window/window_size.hpp"

namespace tmux_cxx {
Result<ClientSnapshot> Server::attach_client(ClientId client, SessionId session) {
    const auto client_entry = state_->clients.find(client);
    if (client_entry == state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    if (!state_->sessions.contains(session)) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    auto &client_state = *client_entry->second;
    if (!client_state.stock_identification_complete) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "client identification is incomplete"});
    }
    const bool control_route =
        client_state.stock_compatibility &&
        client_state.stock_compatibility->mode == protocol::tmux::ConnectionMode::control;
    if (!client_state.terminal && !control_route) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "client has no attachable terminal or control route"});
    }
    if (client_state.terminal) {
        if (auto activation = client_state.terminal->activate(); !activation) {
            return std::unexpected(activation.error());
        }
        if (auto size_update =
                LatestClientWindowSize::apply(*this, session, client_state.terminal->size());
            !size_update) {
            (void)client_state.terminal->restore();
            return std::unexpected(size_update.error());
        }
    }
    if (client_state.session == session) {
        return state_->client_snapshot(client_state);
    }
    if (client_state.session && client_state.session != session) {
        client_state.previous_session = client_state.session;
    }
    client_state.session = session;
    client_state.detached_from.clear();
    client_state.view.invalidate();
    client_state.revision = ++state_->revision;
    return state_->client_snapshot(client_state);
}

AttachClient::Request AttachClient::read(protocol::native::NativePayloadReader &reader) {
    return {ClientId{reader.u64()}, SessionId{reader.u64()}};
}

Result<AttachClient::Response> AttachClient::execute(Server &server, Request request) {
    return server.attach_client(request.client, request.session);
}

protocol::native::OperationReply AttachClient::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.client(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
