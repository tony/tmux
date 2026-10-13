/*
 * Named layouts follow tmux layout-set.c at d1fedcc9a0e98dba.
 * Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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
#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Create a partition node before attaching its uniquely owned children. */
std::unique_ptr<WindowLayout::Cell> partition(WindowLayoutCellKind kind, terminal::CellSize size) {
    auto cell = std::make_unique<WindowLayout::Cell>();
    cell->kind = kind;
    cell->size = size;
    return cell;
}

/** @brief Divide one extent equally, assigning residual cells to earliest logical panes. */
std::unique_ptr<WindowLayout::Cell>
arrange_even(std::span<const PaneId> panes, terminal::CellSize size, WindowLayoutCellKind kind) {
    const auto count = static_cast<std::uint32_t>(panes.size());
    const auto horizontal = kind == WindowLayoutCellKind::left_right;
    const auto minimum = static_cast<std::uint16_t>(count * 2 - 1);
    if (horizontal) {
        size.columns = std::max(size.columns, minimum);
    } else {
        size.rows = std::max(size.rows, minimum);
    }
    const auto extent = horizontal ? size.columns : size.rows;
    const auto each = (extent - count + 1) / count;
    auto remainder = (extent - count + 1) % count;
    auto root = partition(kind, size);
    root->children.reserve(count);
    for (const auto pane : panes) {
        auto child_size = size;
        const auto child_extent = static_cast<std::uint16_t>(each + (remainder != 0 ? 1 : 0));
        if (remainder != 0) {
            --remainder;
        }
        if (horizontal) {
            child_size.columns = child_extent;
        } else {
            child_size.rows = child_extent;
        }
        root->children.push_back(make_layout_leaf(pane, child_size));
    }
    return root;
}

/** @brief Place the logical first pane beside an evenly divided secondary strip using tmux
 * defaults. */
std::unique_ptr<WindowLayout::Cell> arrange_main(std::span<const PaneId> panes,
                                                 terminal::CellSize size, bool horizontal,
                                                 bool mirrored) {
    const auto others = panes.subspan(1);
    const auto strip_minimum = static_cast<std::uint16_t>(others.size() * 2 - 1);
    if (horizontal) {
        size.rows = std::max(size.rows, std::uint16_t{3});
        size.columns = std::max(size.columns, strip_minimum);
    } else {
        size.columns = std::max(size.columns, std::uint16_t{3});
        size.rows = std::max(size.rows, strip_minimum);
    }
    const auto available = static_cast<std::uint32_t>(horizontal ? size.rows : size.columns) - 1;
    const auto main_extent = static_cast<std::uint16_t>(
        std::min(horizontal ? std::uint32_t{24} : std::uint32_t{80}, available - 1));
    const auto other_extent = static_cast<std::uint16_t>(available - main_extent);
    auto main_size = size;
    auto other_size = size;
    if (horizontal) {
        main_size.rows = main_extent;
        other_size.rows = other_extent;
    } else {
        main_size.columns = main_extent;
        other_size.columns = other_extent;
    }
    auto main = make_layout_leaf(panes.front(), main_size);
    auto strip = others.size() == 1 ? make_layout_leaf(others.front(), other_size)
                                    : arrange_even(others, other_size,
                                                   horizontal ? WindowLayoutCellKind::left_right
                                                              : WindowLayoutCellKind::top_bottom);
    auto root = partition(
        horizontal ? WindowLayoutCellKind::top_bottom : WindowLayoutCellKind::left_right, size);
    root->children.push_back(mirrored ? std::move(strip) : std::move(main));
    root->children.push_back(mirrored ? std::move(main) : std::move(strip));
    return root;
}

/** @brief Build row-major tiled panes with residual space in each row's final pane and final row.
 */
std::unique_ptr<WindowLayout::Cell> arrange_tiled(std::span<const PaneId> panes,
                                                  terminal::CellSize size) {
    std::uint32_t rows = 1;
    std::uint32_t columns = 1;
    while (rows * columns < panes.size()) {
        ++rows;
        if (rows * columns < panes.size()) {
            ++columns;
        }
    }
    size.rows = std::max(size.rows, static_cast<std::uint16_t>(rows * 2 - 1));
    size.columns = std::max(size.columns, static_cast<std::uint16_t>(columns * 2 - 1));
    const auto height = (size.rows - rows + 1) / rows;
    const auto width = (size.columns - columns + 1) / columns;
    auto root = partition(WindowLayoutCellKind::top_bottom, size);
    root->children.reserve(rows);
    std::size_t next = 0;
    for (std::uint32_t row = 0; row < rows; ++row) {
        const auto count =
            static_cast<std::uint32_t>(std::min<std::size_t>(columns, panes.size() - next));
        const auto row_height = static_cast<std::uint16_t>(
            row + 1 == rows ? size.rows - (rows - 1) * (height + 1) : height);
        const terminal::CellSize row_size{row_height, size.columns};
        if (count == 1) {
            root->children.push_back(make_layout_leaf(panes[next++], row_size));
            continue;
        }
        auto strip = partition(WindowLayoutCellKind::left_right, row_size);
        strip->children.reserve(count);
        for (std::uint32_t column = 0; column < count; ++column) {
            const auto pane_width = static_cast<std::uint16_t>(
                column + 1 == count ? size.columns - (count - 1) * (width + 1) : width);
            strip->children.push_back(make_layout_leaf(panes[next++], {row_height, pane_width}));
        }
        root->children.push_back(std::move(strip));
    }
    return root;
}

/** @brief Compare partition structure and dimensions, which govern subsequent pane-border movement.
 */
bool same_layout_cells(const WindowLayout::Cell &left, const WindowLayout::Cell &right) {
    if (left.kind != right.kind || left.pane != right.pane || left.size.rows != right.size.rows ||
        left.size.columns != right.size.columns || left.children.size() != right.children.size()) {
        return false;
    }
    for (std::size_t index = 0; index < left.children.size(); ++index) {
        if (!same_layout_cells(*left.children[index], *right.children[index])) {
            return false;
        }
    }
    return true;
}
} // namespace

Result<bool> WindowLayout::arrange(WindowLayoutPreset preset, std::span<const PaneId> pane_order) {
    if (std::to_underlying(preset) >= window_layout_presets.size()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid window layout preset"});
    }
    const auto existing = panes();
    if (pane_order.size() != existing.size()) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "layout pane order does not match membership"});
    }
    std::vector<PaneId> ordered(pane_order.begin(), pane_order.end());
    std::vector<PaneId> identities;
    identities.reserve(existing.size());
    for (const auto &geometry : existing) {
        identities.push_back(geometry.pane);
    }
    std::ranges::sort(ordered);
    std::ranges::sort(identities);
    if (ordered.empty() || ordered.front().value == 0 || ordered != identities ||
        std::ranges::adjacent_find(ordered) != ordered.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid layout pane order"});
    }
    if (pane_order.size() == 1) {
        return false;
    }
    std::unique_ptr<Cell> candidate;
    switch (preset) {
    case WindowLayoutPreset::even_horizontal:
        candidate = arrange_even(pane_order, root_->size, WindowLayoutCellKind::left_right);
        break;
    case WindowLayoutPreset::even_vertical:
        candidate = arrange_even(pane_order, root_->size, WindowLayoutCellKind::top_bottom);
        break;
    case WindowLayoutPreset::main_horizontal:
    case WindowLayoutPreset::main_horizontal_mirrored:
        candidate = arrange_main(pane_order, root_->size, true,
                                 preset == WindowLayoutPreset::main_horizontal_mirrored);
        break;
    case WindowLayoutPreset::main_vertical:
    case WindowLayoutPreset::main_vertical_mirrored:
        candidate = arrange_main(pane_order, root_->size, false,
                                 preset == WindowLayoutPreset::main_vertical_mirrored);
        break;
    case WindowLayoutPreset::tiled:
        candidate = arrange_tiled(pane_order, root_->size);
        break;
    }
    const auto size = candidate->size;
    if (size.rows == 0 || size.columns == 0 || size.rows > 512 || size.columns > 512 ||
        static_cast<std::uint64_t>(size.rows) * size.columns > 16384) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "window layout exceeds terminal capacity"});
    }
    if (same_layout_cells(*root_, *candidate)) {
        return false;
    }
    derive_layout_offsets(*candidate, 0, 0);
    root_ = std::move(candidate);
    return true;
}
} // namespace tmux_cxx
