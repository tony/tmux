#include <tmux_cxx/window_link/operations/list_window_links.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::vector<WindowLinkSnapshot>> Server::window_links(SessionId session_id) const {
    auto session = state_->sessions.find(session_id);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    std::vector<WindowLinkSnapshot> snapshots;
    snapshots.reserve(session->second.windows.size());
    for (const auto &[window_index, link_id] : session->second.windows) {
        (void)window_index;
        snapshots.push_back(state_->window_link_snapshot(state_->window_links.at(link_id)));
    }
    return snapshots;
}
ListWindowLinks::Request ListWindowLinks::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}};
}
Result<ListWindowLinks::Response> ListWindowLinks::execute(Server &server, Request request) {
    return server.window_links(request.session);
}
protocol::native::OperationReply ListWindowLinks::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.size()));
    for (const auto &link : response) {
        reply.window_link(link);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
