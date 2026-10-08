#include <tmux_cxx/pane/operations/select_pane.hpp>

#include "pane/pane.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<PaneSnapshot> Server::select_pane(PaneId pane_id) {
    const auto pane = state_->panes.find(pane_id);
    if (pane == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    const auto window_entry = state_->windows.find(pane->second->window);
    if (window_entry == state_->windows.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "pane's owning window no longer exists"});
    }
    auto &window = window_entry->second;
    const auto geometry = window.layout.pane(pane_id);
    if (!geometry) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "pane is absent from its owning window layout"});
    }
    if (window.active != pane_id) {
        window.active = pane_id;
        window.revision = ++state_->revision;
    }
    return state_->pane_snapshot(window, *geometry);
}

SelectPane::Request SelectPane::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}

Result<SelectPane::Response> SelectPane::execute(Server &server, Request request) {
    return server.select_pane(request.pane);
}

protocol::native::OperationReply SelectPane::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.pane(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
