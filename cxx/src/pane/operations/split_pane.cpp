#include <tmux_cxx/pane/operations/split_pane.hpp>

#include "pane/pane.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <map>
#include <memory>
#include <stdexcept>
#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Restore already resized panes after a split preparation failure. */
Result<void> restore_pane_sizes(std::map<PaneId, std::unique_ptr<Pane>> &panes,
                                std::span<const PaneGeometry> geometries) {
    for (const auto &geometry : geometries) {
        const auto pane = panes.find(geometry.pane);
        if (pane != panes.end()) {
            if (auto restored = pane->second->resize_terminal(geometry.size); !restored) {
                return std::unexpected(TmuxError{
                    restored.error().code,
                    "pane split failed and prior terminal geometry could not be restored"});
            }
        }
    }
    return {};
}
} // namespace

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
    if (auto split = candidate_layout.split(target, new_pane, orientation); !split) {
        return std::unexpected(split.error());
    }
    const auto previous_geometry = window.layout.panes();
    const auto candidate_geometry = candidate_layout.panes();
    for (const auto &geometry : candidate_geometry) {
        if (geometry.pane == new_pane) {
            continue;
        }
        const auto pane = state_->panes.find(geometry.pane);
        if (pane == state_->panes.end()) {
            (void)restore_pane_sizes(state_->panes, previous_geometry);
            return std::unexpected(
                TmuxError{TmuxErrorCode::not_found, "layout pane no longer exists"});
        }
        if (!pane->second->terminal.snapshot().complete) {
            (void)restore_pane_sizes(state_->panes, previous_geometry);
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "pane terminal state is incomplete"});
        }
        if (auto resized = pane->second->resize_terminal(geometry.size); !resized) {
            if (auto restored = restore_pane_sizes(state_->panes, previous_geometry); !restored) {
                return std::unexpected(restored.error());
            }
            return std::unexpected(resized.error());
        }
    }
    const auto new_geometry = candidate_layout.pane(new_pane);
    if (!new_geometry) {
        (void)restore_pane_sizes(state_->panes, previous_geometry);
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "split layout omitted the allocated pane"});
    }
    if (auto spawned = spawn_pane(new_pane, window.id, new_geometry->size, command); !spawned) {
        if (auto restored = restore_pane_sizes(state_->panes, previous_geometry); !restored) {
            return std::unexpected(restored.error());
        }
        return std::unexpected(spawned.error());
    }
    window.layout = std::move(candidate_layout);
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
