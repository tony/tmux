#pragma once

#include "window/layout.hpp"

namespace tmux_cxx {
/** @brief Distinguish a pane leaf from its two ordered partition arrangements. */
enum class WindowLayoutCellKind { leaf, left_right, top_bottom };

/** @brief Retain one owned layout leaf or partition with derived geometric offsets. */
struct WindowLayout::Cell {
    WindowLayoutCellKind kind = WindowLayoutCellKind::leaf; ///< Leaf or owned partition.
    terminal::CellSize size{}; ///< Interior extent governed by the parent invariant.
    std::uint32_t top = 0;     ///< Derived top row within the window.
    std::uint32_t left = 0;    ///< Derived left column within the window.
    PaneId pane{};             ///< Leaf identity; zero for containers.
    std::vector<std::unique_ptr<Cell>> children; ///< Owned children in geometric order.
};

/** @brief Create one pane leaf at the supplied geometry. */
std::unique_ptr<WindowLayout::Cell> make_layout_leaf(PaneId pane, terminal::CellSize size);
/** @brief Derive descendant offsets from extents and one-cell sibling borders. */
void derive_layout_offsets(WindowLayout::Cell &cell, std::uint32_t top, std::uint32_t left);
} // namespace tmux_cxx
