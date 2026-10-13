#include "server/state.hpp"

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

Result<void> Server::State::resize_pane_terminals(std::span<const PaneGeometry> geometries,
                                                  std::span<const PaneGeometry> previous) {
    for (const auto &geometry : geometries) {
        const auto pane = panes.find(geometry.pane);
        if (pane == panes.end()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::not_found, "layout pane no longer exists"});
        }
        if (!pane->second->terminal.snapshot().complete) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "pane terminal state is incomplete"});
        }
    }
    for (const auto &geometry : geometries) {
        auto &pane = *panes.at(geometry.pane);
        if (pane.terminal.snapshot().size == geometry.size) {
            continue;
        }
        if (auto resized = pane.resize_terminal(geometry.size); !resized) {
            if (auto restored = restore_pane_terminal_sizes(previous); !restored) {
                return restored;
            }
            return std::unexpected(resized.error());
        }
    }
    return {};
}
} // namespace tmux_cxx
