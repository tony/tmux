#include <tmux_cxx/pane/operations/list_panes.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::vector<PaneSnapshot>> Server::panes(WindowId window_id) const {
    const auto window = state_->windows.find(window_id);
    if (window == state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    std::vector<PaneSnapshot> snapshots;
    for (const auto &geometry : window->second.layout.panes()) {
        snapshots.push_back(state_->pane_snapshot(window->second, geometry));
    }
    return snapshots;
}

ListPanes::Request ListPanes::read(protocol::native::NativePayloadReader &reader) {
    return {WindowId{reader.u64()}};
}

Result<ListPanes::Response> ListPanes::execute(Server &server, Request request) {
    return server.panes(request.window);
}

protocol::native::OperationReply ListPanes::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.size()));
    for (const auto &pane : response) {
        reply.pane(pane);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
