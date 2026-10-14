#include "server/state.hpp"

#include <type_traits>

namespace tmux_cxx {
Result<void> Server::State::restore_pane_terminal_sizes(std::span<const PaneGeometry> geometries) {
    std::optional<TmuxError> failure;
    for (const auto &geometry : geometries) {
        const auto pane = panes.find(geometry.pane);
        if (pane == panes.end()) {
            continue;
        }
        if (auto restored = pane->second->resize_terminal(geometry.size); !restored && !failure) {
            failure = TmuxError{restored.error().code,
                                "prior pane terminal sizes could not be fully restored"};
        }
    }
    return failure ? Result<void>{std::unexpected(std::move(*failure))} : Result<void>{};
}

Result<std::vector<PendingPaneCopyResize>>
Server::State::resize_pane_terminals(std::span<const PaneGeometry> geometries,
                                     std::span<const PaneGeometry> previous) {
    std::vector<PendingPaneCopyResize> copy_resizes;
    std::vector<PaneGeometry> changed_geometry;
    copy_resizes.reserve(geometries.size());
    changed_geometry.reserve(geometries.size());
    for (const auto &geometry : geometries) {
        const auto pane = panes.find(geometry.pane);
        if (pane == panes.end()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::not_found, "layout pane no longer exists"});
        }
        const auto snapshot = pane->second->terminal.snapshot();
        if (!snapshot.complete) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "pane terminal state is incomplete"});
        }
        if (pane->second->copy_mode && pane->second->copy_mode->state().size != geometry.size) {
            const auto window = windows.find(pane->second->window);
            if (window == windows.end()) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::not_found, "layout window no longer exists"});
            }
            auto keys =
                window_options::effective_mode_keys(window->second.options, global_window_options);
            if (!keys) {
                return std::unexpected(keys.error());
            }
            auto candidate = pane->second->copy_mode->resized(geometry.size, *keys);
            if (!candidate) {
                return std::unexpected(candidate.error());
            }
            copy_resizes.push_back({geometry.pane, std::move(*candidate)});
        }
        if (snapshot.size != geometry.size) {
            changed_geometry.push_back(geometry);
        }
    }
    for (const auto &geometry : changed_geometry) {
        auto &pane = *panes.at(geometry.pane);
        if (auto resized = pane.resize_terminal(geometry.size); !resized) {
            if (auto restored = restore_pane_terminal_sizes(previous); !restored) {
                return std::unexpected(restored.error());
            }
            return std::unexpected(resized.error());
        }
    }
    return copy_resizes;
}

void Server::State::commit_pane_copy_resizes(std::span<PendingPaneCopyResize> candidates) noexcept {
    static_assert(std::is_nothrow_move_assignable_v<std::optional<CopyMode>>);
    static_assert(std::is_nothrow_constructible_v<std::optional<CopyMode>, CopyMode &&>);
    // Candidates address panes retained throughout one serialized geometry mutation.
    for (auto &candidate : candidates) {
        auto &pane = *panes.at(candidate.pane);
        pane.copy_mode = std::move(candidate.mode);
        pane.touch_copy_display();
    }
}
} // namespace tmux_cxx
