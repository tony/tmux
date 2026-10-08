#include <tmux_cxx/window_link/operations/link_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <limits>

namespace tmux_cxx {
Result<WindowLinkSnapshot> Server::link_window(WindowId window_id, SessionId session_id,
                                               std::uint32_t index) {
    auto window = state_->windows.find(window_id);
    auto session = state_->sessions.find(session_id);
    if (window == state_->windows.end() || session == state_->sessions.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "window or session no longer exists"});
    }
    if (index > static_cast<std::uint32_t>(std::numeric_limits<int>::max())) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window index exceeds the tmux index range"});
    }
    if (session->second.windows.contains(index)) {
        return std::unexpected(TmuxError{TmuxErrorCode::duplicate_index,
                                         "window index already exists in this session"});
    }
    const WindowLinkId link_id{state_->next_window_link_identity++};
    const auto revision = ++state_->revision;
    WindowLink window_link{link_id, session_id, window_id, index, revision};
    state_->window_links.emplace(link_id, window_link);
    session->second.windows.emplace(index, link_id);
    session->second.revision = revision;
    window->second.links.insert(link_id);
    return state_->window_link_snapshot(window_link);
}

LinkWindow::Request LinkWindow::read(protocol::native::NativePayloadReader &reader) {
    return {WindowId{reader.u64()}, SessionId{reader.u64()}, reader.u32()};
}
Result<LinkWindow::Response> LinkWindow::execute(Server &server, Request request) {
    return server.link_window(request.window, request.session, request.index);
}
protocol::native::OperationReply LinkWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.window_link(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
