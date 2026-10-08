#pragma once

#include "window/layout.hpp"

#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <vector>

namespace tmux_cxx {
/** @brief Pair one pane's layout geometry with copied terminal state for client composition. */
struct ClientPaneViewSnapshot {
    PaneGeometry geometry;               ///< Pane identity and interior window coordinates.
    terminal::TerminalSnapshot terminal; ///< Copied authoritative terminal state.
    bool active;                         ///< Whether this pane owns the ordinary-client cursor.
};

/** @brief Collect one window's copied layout and pane state for client-specific composition. */
struct ClientWindowViewSnapshot {
    WindowId window;                           ///< Shared window supplying this view.
    std::uint64_t window_revision;             ///< Layout and active-pane freshness.
    terminal::CellSize size;                   ///< Complete shared layout dimensions.
    std::vector<ClientPaneViewSnapshot> panes; ///< Ordered pane values from the layout tree.
};

/** @brief Compose one client's bounded current-state terminal view independently of its TTY route.
 */
class ClientView {
  public:
    /** @brief Report whether source state or outer-terminal dimensions require another redraw. */
    bool needs_redraw(const ClientWindowViewSnapshot &window_view,
                      terminal::CellSize viewport) const;
    /** @brief Compose all pane interiors and basic borders without replaying raw output. */
    Result<std::string> compose_redraw(const ClientWindowViewSnapshot &window_view,
                                       terminal::CellSize viewport) const;
    /** @brief Record a redraw only after its bytes enter the client's bounded output queue. */
    void record_queued_redraw(const ClientWindowViewSnapshot &window_view,
                              terminal::CellSize viewport);
    /** @brief Require a fresh screen-establishing redraw after attachment changes. */
    void invalidate();

  private:
    /** @brief Retain only pane identity and terminal freshness after queue admission. */
    struct QueuedPaneRevision {
        PaneId pane;                     ///< Pane represented by queued redraw bytes.
        std::uint64_t terminal_revision; ///< Terminal revision represented for that pane.
    };

    WindowId queued_window_{};                 ///< Window represented by queued redraw bytes.
    std::uint64_t queued_window_revision_ = 0; ///< Layout and active-pane revision represented.
    std::vector<QueuedPaneRevision>
        queued_pane_revisions_;            ///< Pane terminal baselines represented in layout order.
    terminal::CellSize queued_viewport_{}; ///< Outer dimensions represented by queued redraw bytes.
    bool screen_entry_queued_ =
        false; ///< The output route has accepted alternate-screen entry bytes.
};
} // namespace tmux_cxx
