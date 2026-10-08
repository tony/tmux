#include <tmux_cxx/window_link/operations/unlink_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::unlink_window(WindowLinkId window_link_id) {
    auto window_link = state_->window_links.find(window_link_id);
    if (window_link == state_->window_links.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "window membership no longer exists"});
    }
    auto &session = state_->sessions.at(window_link->second.session);
    if (session.windows.size() == 1) {
        return kill_session(session.id);
    }
    state_->remove_window_link(window_link_id);
    return {};
}
UnlinkWindow::Request UnlinkWindow::read(protocol::native::NativePayloadReader &reader) {
    return {WindowLinkId{reader.u64()}};
}
Result<UnlinkWindow::Response> UnlinkWindow::execute(Server &server, Request request) {
    return server.unlink_window(request.window_link);
}
protocol::native::OperationReply UnlinkWindow::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
