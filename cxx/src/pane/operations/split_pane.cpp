#include <tmux_cxx/pane/operations/split_pane.hpp>

#include "pane/pane.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<PaneSnapshot> Server::split_pane(PaneId target, PaneSplitOrientation orientation,
                                        std::string command) {
    const auto target_pane = state_->panes.find(target);
    if (target_pane == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    const auto window_entry = state_->windows.find(target_pane->second->window);
    if (window_entry == state_->windows.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "pane's owning window no longer exists"});
    }
    if (command.empty() || command.find('\0') != command.npos ||
        (orientation != PaneSplitOrientation::left_right &&
         orientation != PaneSplitOrientation::top_bottom)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid pane split or command"});
    }
    auto &window = window_entry->second;
    auto candidate_layout = window.layout;
    const PaneId new_pane{state_->next_pane_identity++};
    auto candidate_order = window.pane_order;
    const auto target_position = std::ranges::find(candidate_order, target);
    if (target_position == candidate_order.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "split target is absent from window pane order"});
    }
    candidate_order.insert(std::next(target_position), new_pane);
    if (auto split = candidate_layout.split(target, new_pane, orientation); !split) {
        return std::unexpected(split.error());
    }
    const auto previous_geometry = window.layout.panes();
    const auto new_geometry = candidate_layout.pane(new_pane);
    if (!new_geometry) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "split layout omitted the allocated pane"});
    }
    auto existing_geometry = candidate_layout.panes();
    std::erase_if(
        existing_geometry, /** @brief Exclude the new leaf until its program has started. */
        [new_pane](const PaneGeometry &geometry) { return geometry.pane == new_pane; });
    if (auto resized = state_->resize_pane_terminals(existing_geometry, previous_geometry);
        !resized) {
        return std::unexpected(resized.error());
    }
    if (auto spawned = spawn_pane(new_pane, window.id, new_geometry->size, command); !spawned) {
        if (auto restored = state_->restore_pane_terminal_sizes(previous_geometry); !restored) {
            return std::unexpected(restored.error());
        }
        return std::unexpected(spawned.error());
    }
    window.layout = std::move(candidate_layout);
    window.pane_order = std::move(candidate_order);
    window.active = new_pane;
    window.revision = ++state_->revision;
    return state_->pane_snapshot(window, *new_geometry);
}

SplitPane::Request SplitPane::read(protocol::native::NativePayloadReader &reader) {
    const auto pane = PaneId{reader.u64()};
    const auto orientation = reader.u16();
    if (orientation > std::to_underlying(PaneSplitOrientation::top_bottom)) {
        throw std::invalid_argument("invalid pane split orientation");
    }
    return {pane, static_cast<PaneSplitOrientation>(orientation), reader.string()};
}

Result<SplitPane::Response> SplitPane::execute(Server &server, Request request) {
    return server.split_pane(request.pane, request.orientation, std::move(request.command));
}

protocol::native::OperationReply SplitPane::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.pane(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
