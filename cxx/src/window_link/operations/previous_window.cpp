#include <tmux_cxx/window_link/operations/previous_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <iterator>
#include <utility>

namespace tmux_cxx {
Result<SessionSnapshot> Server::previous_window(SessionId session_id) {
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
    const auto target = std::prev(current == memberships.begin() ? memberships.end() : current);
    return select_window(target->second);
}

PreviousWindow::Request PreviousWindow::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}};
}
Result<PreviousWindow::Response> PreviousWindow::execute(Server &server, Request request) {
    return server.previous_window(request.session);
}
protocol::native::OperationReply PreviousWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.session(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
