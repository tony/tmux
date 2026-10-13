#include <tmux_cxx/pane/operations/resize_pane.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <stdexcept>

namespace tmux_cxx {
Result<PaneSnapshot> Server::resize_pane(PaneId pane_id, PaneResizeDirection direction,
                                         std::uint32_t cells) {
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
    auto candidate = window.layout;
    const auto changed = candidate.resize_pane(pane_id, direction, cells);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (*changed) {
        if (auto resized = state_->resize_pane_terminals(candidate.panes(), window.layout.panes());
            !resized) {
            return std::unexpected(resized.error());
        }
        window.layout = std::move(candidate);
        window.revision = ++state_->revision;
    }
    return state_->pane_snapshot(window, *window.layout.pane(pane_id));
}

ResizePane::Request ResizePane::read(protocol::native::NativePayloadReader &reader) {
    const auto pane = PaneId{reader.u64()};
    const auto direction = reader.u16();
    if (direction > std::to_underlying(PaneResizeDirection::down)) {
        throw std::invalid_argument("invalid pane resize direction");
    }
    return {pane, static_cast<PaneResizeDirection>(direction), reader.u32()};
}

Result<ResizePane::Response> ResizePane::execute(Server &server, Request request) {
    return server.resize_pane(request.pane, request.direction, request.cells);
}

protocol::native::OperationReply ResizePane::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.pane(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
