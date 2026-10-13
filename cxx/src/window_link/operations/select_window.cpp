#include <tmux_cxx/window_link/operations/select_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "window/window_size.hpp"

namespace tmux_cxx {
Result<SessionSnapshot> Server::select_window(WindowLinkId window_link) {
    const auto link = state_->window_links.find(window_link);
    if (link == state_->window_links.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "window membership no longer exists"});
    }
    auto &session = state_->sessions.at(link->second.session);
    if (session.current != window_link) {
        const Client *latest_client = nullptr;
        for (const auto &[client_id, client] : state_->clients) {
            (void)client_id;
            if (client->session == session.id && client->terminal && client->terminal->active() &&
                (!latest_client || client->revision > latest_client->revision)) {
                latest_client = client.get();
            }
        }
        if (latest_client) {
            if (auto resized = LatestClientWindowSize::apply_to_window(
                    *this, link->second.window, latest_client->terminal->size());
                !resized) {
                return std::unexpected(resized.error());
            }
        }
        session.current = window_link;
        session.revision = ++state_->revision;
    }
    return state_->session_snapshot(session);
}
SelectWindow::Request SelectWindow::read(protocol::native::NativePayloadReader &reader) {
    return {WindowLinkId{reader.u64()}};
}
Result<SelectWindow::Response> SelectWindow::execute(Server &server, Request request) {
    return server.select_window(request.window_link);
}
protocol::native::OperationReply SelectWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.session(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
