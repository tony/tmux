#include <tmux_cxx/window_link/operations/next_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <iterator>
#include <utility>

namespace tmux_cxx {
Result<SessionSnapshot> Server::next_window(SessionId session_id) {
    const auto session = state_->sessions.find(session_id);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    const auto &memberships = session->second.windows;
    const auto current = memberships.find(state_->window_links.at(session->second.current).index);
    if (current == memberships.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "session's current window membership is stale"});
    }
    auto target = std::next(current);
    if (target == memberships.end()) {
        target = memberships.begin();
    }
    return select_window(target->second);
}

NextWindow::Request NextWindow::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}};
}
Result<NextWindow::Response> NextWindow::execute(Server &server, Request request) {
    return server.next_window(request.session);
}
protocol::native::OperationReply NextWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.session(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
