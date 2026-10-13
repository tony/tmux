#pragma once

#include <tmux_cxx/terminal/terminal_snapshot.hpp>

#include <cstdint>
#include <vector>

namespace tmux_cxx {
/** @brief Copy a pane's displayed cells and cursor independently of its program terminal facts. */
struct PaneScreenSnapshot {
    terminal::CellSize size;       ///< Display dimensions in character cells.
    terminal::CellPosition cursor; ///< Zero-based cursor within the displayed cells.
    bool cursor_visible;           ///< Whether the displayed cursor should be shown.
    bool complete;                 ///< False when the selected display content is incomplete.
    std::uint64_t revision;        ///< Freshness token for the pane's chosen display.
    std::vector<terminal::TerminalCell>
        cells; ///< Owned row-major displayed cells; exactly rows multiplied by columns.
};
} // namespace tmux_cxx
