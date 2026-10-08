#include "window/layout.hpp"

#include <algorithm>
#include <limits>
#include <stdexcept>

namespace tmux_cxx {
namespace {
constexpr std::size_t max_layout_panes = 256; ///< Bound tree depth, traversal, and process count.

/** @brief Distinguish layout leaves from the two ordered container arrangements. */
enum class CellKind { leaf, left_right, top_bottom };

/** @brief Translate the public split choice into its owning container kind. */
CellKind container_kind(PaneSplitOrientation orientation) {
    return orientation == PaneSplitOrientation::left_right ? CellKind::left_right
                                                           : CellKind::top_bottom;
}
} // namespace

/** @brief Retain one layout leaf or ordered container with derived geometry. */
struct WindowLayout::Cell {
    CellKind kind = CellKind::leaf; ///< Leaf or arrangement owned by this cell.
    terminal::CellSize size{};      ///< Interior extent governed by the parent invariant.
    std::uint32_t top = 0;          ///< Derived top row within the window.
    std::uint32_t left = 0;         ///< Derived left column within the window.
    PaneId pane{};                  ///< Leaf pane; zero for containers.
    std::vector<std::unique_ptr<Cell>> children; ///< Ordered children; empty for leaves.
};

namespace {
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
    if (cell.kind == CellKind::leaf) {
        return 1;
    }
    std::size_t count = 0;
    for (const auto &child : cell.children) {
        count += leaf_count(*child);
    }
    return count;
}

/** @brief Recompute every descendant offset from sizes and one-cell sibling borders. */
void derive_offsets(WindowLayout::Cell &cell, std::uint32_t top, std::uint32_t left) {
    cell.top = top;
    cell.left = left;
    std::uint32_t child_top = top;
    std::uint32_t child_left = left;
    for (auto &child : cell.children) {
        derive_offsets(*child, child_top, child_left);
        if (cell.kind == CellKind::left_right) {
            child_left += child->size.columns + 1;
        } else if (cell.kind == CellKind::top_bottom) {
            child_top += child->size.rows + 1;
        }
    }
}

/** @brief Create a leaf at the supplied pane and geometry. */
std::unique_ptr<WindowLayout::Cell> leaf(PaneId pane, terminal::CellSize size) {
    auto cell = std::make_unique<WindowLayout::Cell>();
    cell->size = size;
    cell->pane = pane;
    return cell;
}

/** @brief Divide the target leaf or insert beside it in a matching parent container. */
bool split_cell(WindowLayout::Cell &cell, PaneId target, PaneId new_pane, CellKind arrangement,
                bool &too_small) {
    if (cell.kind == CellKind::leaf) {
        if (cell.pane != target) {
            return false;
        }
        const auto extent =
            arrangement == CellKind::left_right ? cell.size.columns : cell.size.rows;
        if (extent < 3) {
            too_small = true;
            return false;
        }
        const auto new_extent = (extent + 1) / 2 - 1;
        const auto old_extent = extent - new_extent - 1;
        auto old_size = cell.size;
        auto new_size = cell.size;
        if (arrangement == CellKind::left_right) {
            old_size.columns = old_extent;
            new_size.columns = new_extent;
        } else {
            old_size.rows = old_extent;
            new_size.rows = new_extent;
        }
        const auto old_pane = cell.pane;
        cell.kind = arrangement;
        cell.pane = {};
        cell.children.push_back(leaf(old_pane, old_size));
        cell.children.push_back(leaf(new_pane, new_size));
        return true;
    }
    for (std::size_t index = 0; index < cell.children.size(); ++index) {
        auto &child = cell.children[index];
        if (cell.kind == arrangement && child->kind == CellKind::leaf && child->pane == target) {
            const auto extent =
                arrangement == CellKind::left_right ? child->size.columns : child->size.rows;
            if (extent < 3) {
                too_small = true;
                return false;
            }
            const auto new_extent = (extent + 1) / 2 - 1;
            if (arrangement == CellKind::left_right) {
                child->size.columns = extent - new_extent - 1;
                auto new_size = child->size;
                new_size.columns = new_extent;
                cell.children.insert(cell.children.begin() + static_cast<std::ptrdiff_t>(index + 1),
                                     leaf(new_pane, new_size));
            } else {
                child->size.rows = extent - new_extent - 1;
                auto new_size = child->size;
                new_size.rows = new_extent;
                cell.children.insert(cell.children.begin() + static_cast<std::ptrdiff_t>(index + 1),
                                     leaf(new_pane, new_size));
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
    if (cell.kind == CellKind::leaf) {
        return {1, 1};
    }
    terminal::CellSize minimum{};
    for (const auto &child : cell.children) {
        const auto child_minimum = minimum_size(*child);
        if (cell.kind == CellKind::left_right) {
            minimum.rows = std::max(minimum.rows, child_minimum.rows);
            minimum.columns += child_minimum.columns;
        } else {
            minimum.rows += child_minimum.rows;
            minimum.columns = std::max(minimum.columns, child_minimum.columns);
        }
    }
    if (cell.kind == CellKind::left_right) {
        minimum.columns += static_cast<std::uint32_t>(cell.children.size() - 1);
    } else {
        minimum.rows += static_cast<std::uint32_t>(cell.children.size() - 1);
    }
    return minimum;
}

/** @brief Select the row or column extent changed by a whole-window resize. */
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
    return (axis == LayoutAxis::rows && cell.kind == CellKind::top_bottom) ||
           (axis == LayoutAxis::columns && cell.kind == CellKind::left_right);
}

/** @brief Apply one axis delta using tmux's ordered round-robin subtree distribution. */
bool resize_axis(WindowLayout::Cell &cell, LayoutAxis axis, std::int32_t change) {
    if (change == 0) {
        return true;
    }
    const auto changed_extent =
        static_cast<std::uint16_t>(static_cast<std::int32_t>(extent(cell, axis)) + change);
    if (cell.kind == CellKind::leaf) {
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

/** @brief Append leaf geometry in stable depth-first pane-list order. */
void collect_panes(const WindowLayout::Cell &cell, std::vector<PaneGeometry> &panes) {
    if (cell.kind == CellKind::leaf) {
        panes.push_back({cell.pane, cell.top, cell.left, cell.size});
        return;
    }
    for (const auto &child : cell.children) {
        collect_panes(*child, panes);
    }
}
} // namespace

WindowLayout::WindowLayout(PaneId pane, terminal::CellSize size) : root_(leaf(pane, size)) {
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
    derive_offsets(*root_, 0, 0);
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
    derive_offsets(*candidate, 0, 0);
    root_ = std::move(candidate);
    return resized;
}

std::vector<PaneGeometry> WindowLayout::panes() const {
    std::vector<PaneGeometry> result;
    result.reserve(leaf_count(*root_));
    collect_panes(*root_, result);
    return result;
}

std::optional<PaneGeometry> WindowLayout::pane(PaneId pane_id) const {
    const auto all_panes = panes();
    auto pane = std::ranges::find(all_panes, pane_id, &PaneGeometry::pane);
    return pane == all_panes.end() ? std::nullopt : std::optional(*pane);
}

terminal::CellSize WindowLayout::size() const { return root_->size; }
} // namespace tmux_cxx
