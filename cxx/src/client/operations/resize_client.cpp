#include <tmux_cxx/client/operations/resize_client.hpp>

#include "client/client.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "window/window_size.hpp"

namespace tmux_cxx {
Result<ClientSnapshot> Server::resize_client(ClientId client) {
    const auto client_entry = state_->clients.find(client);
    if (client_entry == state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    auto &client_state = *client_entry->second;
    if (!client_state.session || !client_state.terminal || !client_state.terminal->active()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "client is not attached to a terminal"});
    }
    auto measured_size = client_state.terminal->measure();
    if (!measured_size) {
        return std::unexpected(measured_size.error());
    }
    if (auto size_update =
            LatestClientWindowSize::apply(*this, *client_state.session, *measured_size);
        !size_update) {
        return std::unexpected(size_update.error());
    }
    client_state.view.invalidate();
    client_state.revision = ++state_->revision;
    return state_->client_snapshot(client_state);
}

ResizeClient::Request ResizeClient::read(protocol::native::NativePayloadReader &reader) {
    return {ClientId{reader.u64()}};
}

Result<ResizeClient::Response> ResizeClient::execute(Server &server, Request request) {
    return server.resize_client(request.client);
}

protocol::native::OperationReply ResizeClient::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.client(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
