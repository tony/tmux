/*
 * Pane border resizing follows tmux layout.c at d1fedcc9a0e98dba.
 * Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
 * Copyright (c) 2016 Stephen Kent <smkent@smkent.net>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
 * IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
 * OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

#include "window/layout_cell.hpp"

#include <algorithm>
#include <limits>
#include <stdexcept>

namespace tmux_cxx {
namespace {
constexpr std::size_t max_layout_panes = 256; ///< Bound tree depth, traversal, and process count.

/** @brief Translate the public split choice into its owning container kind. */
WindowLayoutCellKind container_kind(PaneSplitOrientation orientation) {
    return orientation == PaneSplitOrientation::left_right ? WindowLayoutCellKind::left_right
                                                           : WindowLayoutCellKind::top_bottom;
}
/** @brief Deep-copy one layout subtree while retaining its derived geometry. */
std::unique_ptr<WindowLayout::Cell> clone_cell(const WindowLayout::Cell &source) {
    auto copy = std::make_unique<WindowLayout::Cell>();
    copy->kind = source.kind;
    copy->size = source.size;
    copy->top = source.top;
    copy->left = source.left;
    copy->pane = source.pane;
    copy->children.reserve(source.children.size());
    for (const auto &child : source.children) {
        copy->children.push_back(clone_cell(*child));
    }
    return copy;
}

/** @brief Count leaves without materializing their geometry. */
std::size_t leaf_count(const WindowLayout::Cell &cell) {
    if (cell.kind == WindowLayoutCellKind::leaf) {
        return 1;
    }
    std::size_t count = 0;
    for (const auto &child : cell.children) {
        count += leaf_count(*child);
    }
    return count;
}

} // namespace

void derive_layout_offsets(WindowLayout::Cell &cell, std::uint32_t top, std::uint32_t left) {
    cell.top = top;
    cell.left = left;
    std::uint32_t child_top = top;
    std::uint32_t child_left = left;
    for (auto &child : cell.children) {
        derive_layout_offsets(*child, child_top, child_left);
        if (cell.kind == WindowLayoutCellKind::left_right) {
            child_left += child->size.columns + 1;
        } else if (cell.kind == WindowLayoutCellKind::top_bottom) {
            child_top += child->size.rows + 1;
        }
    }
}

std::unique_ptr<WindowLayout::Cell> make_layout_leaf(PaneId pane, terminal::CellSize size) {
    auto cell = std::make_unique<WindowLayout::Cell>();
    cell->size = size;
    cell->pane = pane;
    return cell;
}

namespace {
/** @brief Divide the target leaf or insert beside it in a matching parent container. */
bool split_cell(WindowLayout::Cell &cell, PaneId target, PaneId new_pane,
                WindowLayoutCellKind arrangement, bool &too_small) {
    if (cell.kind == WindowLayoutCellKind::leaf) {
        if (cell.pane != target) {
            return false;
        }
        const auto extent =
            arrangement == WindowLayoutCellKind::left_right ? cell.size.columns : cell.size.rows;
        if (extent < 3) {
            too_small = true;
            return false;
        }
        const auto new_extent = (extent + 1) / 2 - 1;
        const auto old_extent = extent - new_extent - 1;
        auto old_size = cell.size;
        auto new_size = cell.size;
        if (arrangement == WindowLayoutCellKind::left_right) {
            old_size.columns = old_extent;
            new_size.columns = new_extent;
        } else {
            old_size.rows = old_extent;
            new_size.rows = new_extent;
        }
        const auto old_pane = cell.pane;
        cell.kind = arrangement;
        cell.pane = {};
        cell.children.push_back(make_layout_leaf(old_pane, old_size));
        cell.children.push_back(make_layout_leaf(new_pane, new_size));
        return true;
    }
    for (std::size_t index = 0; index < cell.children.size(); ++index) {
        auto &child = cell.children[index];
        if (cell.kind == arrangement && child->kind == WindowLayoutCellKind::leaf &&
            child->pane == target) {
            const auto extent = arrangement == WindowLayoutCellKind::left_right
                                    ? child->size.columns
                                    : child->size.rows;
            if (extent < 3) {
                too_small = true;
                return false;
            }
            const auto new_extent = (extent + 1) / 2 - 1;
            if (arrangement == WindowLayoutCellKind::left_right) {
                child->size.columns = extent - new_extent - 1;
                auto new_size = child->size;
                new_size.columns = new_extent;
                cell.children.insert(cell.children.begin() + static_cast<std::ptrdiff_t>(index + 1),
                                     make_layout_leaf(new_pane, new_size));
            } else {
                child->size.rows = extent - new_extent - 1;
                auto new_size = child->size;
                new_size.rows = new_extent;
                cell.children.insert(cell.children.begin() + static_cast<std::ptrdiff_t>(index + 1),
                                     make_layout_leaf(new_pane, new_size));
            }
            return true;
        }
        if (split_cell(*child, target, new_pane, arrangement, too_small)) {
            return true;
        }
        if (too_small) {
            return false;
        }
    }
    return false;
}

/** @brief Compute the minimum root size that retains every leaf and sibling border. */
terminal::CellSize minimum_size(const WindowLayout::Cell &cell) {
    if (cell.kind == WindowLayoutCellKind::leaf) {
        return {1, 1};
    }
    terminal::CellSize minimum{};
    for (const auto &child : cell.children) {
        const auto child_minimum = minimum_size(*child);
        if (cell.kind == WindowLayoutCellKind::left_right) {
            minimum.rows = std::max(minimum.rows, child_minimum.rows);
            minimum.columns += child_minimum.columns;
        } else {
            minimum.rows += child_minimum.rows;
            minimum.columns = std::max(minimum.columns, child_minimum.columns);
        }
    }
    if (cell.kind == WindowLayoutCellKind::left_right) {
        minimum.columns += static_cast<std::uint32_t>(cell.children.size() - 1);
    } else {
        minimum.rows += static_cast<std::uint32_t>(cell.children.size() - 1);
    }
    return minimum;
}

/** @brief Select the row or column extent changed by a resize. */
enum class LayoutAxis { rows, columns };

/** @brief Read one cell extent without conflating rows and columns. */
std::uint16_t extent(const WindowLayout::Cell &cell, LayoutAxis axis) {
    return axis == LayoutAxis::rows ? cell.size.rows : cell.size.columns;
}

/** @brief Replace one cell extent after the caller preserves its recursive minimum. */
void set_extent(WindowLayout::Cell &cell, LayoutAxis axis, std::uint16_t value) {
    if (axis == LayoutAxis::rows) {
        cell.size.rows = value;
    } else {
        cell.size.columns = value;
    }
}

/** @brief Report whether this container partitions its children along the changed axis. */
bool partitions_axis(const WindowLayout::Cell &cell, LayoutAxis axis) {
    return (axis == LayoutAxis::rows && cell.kind == WindowLayoutCellKind::top_bottom) ||
           (axis == LayoutAxis::columns && cell.kind == WindowLayoutCellKind::left_right);
}

/** @brief Borrow a sibling boundary only for the lifetime of its candidate layout. */
struct PaneBorder {
    WindowLayout::Cell *parent; ///< Container partitioning the requested axis.
    std::size_t before;         ///< Child preceding the movable border.
};
/** @brief Distinguish a missing pane from one with no border along the requested axis. */
struct PaneBorderSearch {
    bool found;                       ///< Target leaf belongs to this layout.
    std::optional<PaneBorder> border; ///< Nearest ancestor partition, if any.
};
/** @brief Locate the closest matching ancestor and use the preceding border for a final child. */
PaneBorderSearch pane_border(WindowLayout::Cell &cell, PaneId pane, LayoutAxis axis) {
    if (cell.kind == WindowLayoutCellKind::leaf) {
        return {cell.pane == pane, std::nullopt};
    }
    for (std::size_t index = 0; index < cell.children.size(); ++index) {
        auto target = pane_border(*cell.children[index], pane, axis);
        if (target.found) {
            if (!target.border && partitions_axis(cell, axis)) {
                target.border =
                    PaneBorder{&cell, index + 1 == cell.children.size() ? index - 1 : index};
            }
            return target;
        }
    }
    return {false, std::nullopt};
}
/** @brief Count removable cells without violating any descendant's one-cell minimum. */
std::uint32_t spare_extent(const WindowLayout::Cell &cell, LayoutAxis axis) {
    const auto minimum = minimum_size(cell);
    return extent(cell, axis) - (axis == LayoutAxis::rows ? minimum.rows : minimum.columns);
}
/** @brief Choose tmux's ordered donor: following then preceding for growth, preceding for shrink.
 */
std::optional<std::size_t> border_donor(const PaneBorder &border, LayoutAxis axis, bool grow) {
    const auto &children = border.parent->children;
    if (grow) {
        for (std::size_t index = border.before + 1; index < children.size(); ++index) {
            if (spare_extent(*children[index], axis) != 0) {
                return index;
            }
        }
    }
    for (std::size_t end = border.before + (grow ? 0 : 1); end != 0; --end) {
        if (spare_extent(*children[end - 1], axis) != 0) {
            return end - 1;
        }
    }
    return std::nullopt;
}

/** @brief Apply one axis delta using tmux's ordered round-robin subtree distribution. */
bool resize_axis(WindowLayout::Cell &cell, LayoutAxis axis, std::int32_t change) {
    if (change == 0) {
        return true;
    }
    const auto changed_extent =
        static_cast<std::uint16_t>(static_cast<std::int32_t>(extent(cell, axis)) + change);
    if (cell.kind == WindowLayoutCellKind::leaf) {
        set_extent(cell, axis, changed_extent);
        return true;
    }
    if (!partitions_axis(cell, axis)) {
        for (auto &child : cell.children) {
            if (!resize_axis(*child, axis, change)) {
                return false;
            }
        }
        set_extent(cell, axis, changed_extent);
        return true;
    }

    const auto step = change > 0 ? 1 : -1;
    const auto units = static_cast<std::uint32_t>(change > 0 ? change : -change);
    std::size_t next_child = 0;
    for (std::uint32_t unit = 0; unit < units; ++unit) {
        std::optional<std::size_t> selected;
        for (std::size_t offset = 0; offset < cell.children.size(); ++offset) {
            const auto index = (next_child + offset) % cell.children.size();
            const auto child_minimum = minimum_size(*cell.children[index]);
            const auto minimum_extent =
                axis == LayoutAxis::rows ? child_minimum.rows : child_minimum.columns;
            if (step > 0 || extent(*cell.children[index], axis) > minimum_extent) {
                selected = index;
                break;
            }
        }
        if (!selected || !resize_axis(*cell.children[*selected], axis, step)) {
            return false;
        }
        next_child = (*selected + 1) % cell.children.size();
    }
    set_extent(cell, axis, changed_extent);
    return true;
}

/** @brief Append leaf geometry in depth-first geometric traversal order. */
void collect_panes(const WindowLayout::Cell &cell, std::vector<PaneGeometry> &panes) {
    if (cell.kind == WindowLayoutCellKind::leaf) {
        panes.push_back({cell.pane, cell.top, cell.left, cell.size});
        return;
    }
    for (const auto &child : cell.children) {
        collect_panes(*child, panes);
    }
}
} // namespace

WindowLayout::WindowLayout(PaneId pane, terminal::CellSize size)
    : root_(make_layout_leaf(pane, size)) {
    if (pane.value == 0 || size.rows == 0 || size.columns == 0 || size.rows > 512 ||
        size.columns > 512 || static_cast<std::uint64_t>(size.rows) * size.columns > 16384) {
        throw std::invalid_argument("invalid initial window layout");
    }
}
WindowLayout::WindowLayout(const WindowLayout &other) : root_(clone_cell(*other.root_)) {}
WindowLayout &WindowLayout::operator=(const WindowLayout &other) {
    if (this != &other) {
        root_ = clone_cell(*other.root_);
    }
    return *this;
}
WindowLayout::WindowLayout(WindowLayout &&) noexcept = default;
WindowLayout &WindowLayout::operator=(WindowLayout &&) noexcept = default;
WindowLayout::~WindowLayout() = default;

Result<void> WindowLayout::split(PaneId target, PaneId new_pane, PaneSplitOrientation orientation) {
    if (target.value == 0 || new_pane.value == 0 || target == new_pane ||
        leaf_count(*root_) >= max_layout_panes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid or excessive pane split"});
    }
    bool too_small = false;
    if (!split_cell(*root_, target, new_pane, container_kind(orientation), too_small)) {
        return std::unexpected(
            TmuxError{too_small ? TmuxErrorCode::invalid_argument : TmuxErrorCode::not_found,
                      too_small ? "pane has no space for a new pane" : "pane no longer exists"});
    }
    derive_layout_offsets(*root_, 0, 0);
    return {};
}

Result<terminal::CellSize> WindowLayout::resize(terminal::CellSize requested) {
    if (requested.rows == 0 || requested.columns == 0 || requested.rows > 512 ||
        requested.columns > 512 ||
        static_cast<std::uint64_t>(requested.rows) * requested.columns > 16384) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid window layout dimensions"});
    }
    const auto minimum = minimum_size(*root_);
    const terminal::CellSize resized{std::max(requested.rows, minimum.rows),
                                     std::max(requested.columns, minimum.columns)};
    if (static_cast<std::uint64_t>(resized.rows) * resized.columns > 16384) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "minimum layout exceeds terminal capacity"});
    }
    auto candidate = clone_cell(*root_);
    const auto row_change = static_cast<std::int32_t>(resized.rows) - candidate->size.rows;
    const auto column_change = static_cast<std::int32_t>(resized.columns) - candidate->size.columns;
    if (!resize_axis(*candidate, LayoutAxis::rows, row_change) ||
        !resize_axis(*candidate, LayoutAxis::columns, column_change)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "layout resize could not preserve subtree minima"});
    }
    derive_layout_offsets(*candidate, 0, 0);
    root_ = std::move(candidate);
    return resized;
}

std::vector<PaneGeometry> WindowLayout::panes() const {
    std::vector<PaneGeometry> result;
    result.reserve(leaf_count(*root_));
    collect_panes(*root_, result);
    return result;
}

Result<bool> WindowLayout::resize_pane(PaneId pane, PaneResizeDirection direction,
                                       std::uint32_t cells) {
    if (direction != PaneResizeDirection::left && direction != PaneResizeDirection::right &&
        direction != PaneResizeDirection::up && direction != PaneResizeDirection::down) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid pane resize direction"});
    }
    const auto axis =
        direction == PaneResizeDirection::left || direction == PaneResizeDirection::right
            ? LayoutAxis::columns
            : LayoutAxis::rows;
    auto candidate = clone_cell(*root_);
    const auto target = pane_border(*candidate, pane, axis);
    if (!target.found) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    if (!target.border || cells == 0) {
        return false;
    }
    const auto &border = *target.border;
    const auto grow =
        direction == PaneResizeDirection::right || direction == PaneResizeDirection::down;
    auto &receiver = *border.parent->children[border.before + (grow ? 0 : 1)];
    const auto requested = cells;
    while (cells != 0) {
        const auto donor_index = border_donor(border, axis, grow);
        if (!donor_index) {
            break;
        }
        auto &donor = *border.parent->children[*donor_index];
        const auto transferred = std::min(cells, spare_extent(donor, axis));
        const auto change = static_cast<std::int32_t>(transferred);
        if (!resize_axis(receiver, axis, change) || !resize_axis(donor, axis, -change)) {
            return std::unexpected(TmuxError{TmuxErrorCode::protocol,
                                             "pane resize could not preserve subtree minima"});
        }
        cells -= transferred;
    }
    if (cells == requested) {
        return false;
    }
    derive_layout_offsets(*candidate, 0, 0);
    root_ = std::move(candidate);
    return true;
}

std::optional<PaneGeometry> WindowLayout::pane(PaneId pane_id) const {
    const auto all_panes = panes();
    auto pane = std::ranges::find(all_panes, pane_id, &PaneGeometry::pane);
    return pane == all_panes.end() ? std::nullopt : std::optional(*pane);
}

terminal::CellSize WindowLayout::size() const { return root_->size; }
} // namespace tmux_cxx
