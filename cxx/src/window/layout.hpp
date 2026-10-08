#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <memory>
#include <optional>
#include <vector>

namespace tmux_cxx {
/** @brief Describe one layout leaf without exposing the owning tree. */
struct PaneGeometry {
    PaneId pane;             ///< Pane occupying this leaf.
    std::uint32_t top;       ///< Derived top row within the window.
    std::uint32_t left;      ///< Derived left column within the window.
    terminal::CellSize size; ///< Interior dimensions excluding layout borders.
};

/** @brief Own a window's ordered pane-partition tree and derived leaf geometry. */
class WindowLayout {
  public:
    /** @brief Internal tree cell exposed only to this implementation unit's algorithms. */
    struct Cell;
    /** @brief Create a single-leaf layout covering the complete window. */
    WindowLayout(PaneId pane, terminal::CellSize size);
    /** @brief Deep-copy layout ownership without sharing mutable cells. */
    WindowLayout(const WindowLayout &other);
    /** @brief Replace this layout with an independent deep copy. */
    WindowLayout &operator=(const WindowLayout &other);
    /** @brief Transfer complete tree ownership. */
    WindowLayout(WindowLayout &&) noexcept;
    /** @brief Replace this layout by transferring complete tree ownership. */
    WindowLayout &operator=(WindowLayout &&) noexcept;
    /** @brief Release the complete layout tree. */
    ~WindowLayout();
    /** @brief Divide one leaf equally after reserving a one-cell border. */
    Result<void> split(PaneId target, PaneId new_pane, PaneSplitOrientation orientation);
    /** @brief Refit the tree to bounded window dimensions while preserving every leaf minimum. */
    Result<terminal::CellSize> resize(terminal::CellSize requested);
    /** @brief Copy leaf geometry in pane-list order. */
    std::vector<PaneGeometry> panes() const;
    /** @brief Copy one pane's geometry when that pane belongs to this layout. */
    std::optional<PaneGeometry> pane(PaneId pane) const;
    /** @brief Return the root dimensions currently owned by this layout. */
    terminal::CellSize size() const;

  private:
    std::unique_ptr<Cell> root_; ///< Exclusive root of the window's partition tree.
};
} // namespace tmux_cxx
