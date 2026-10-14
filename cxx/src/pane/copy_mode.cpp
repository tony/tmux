/*
 * Character selection, navigation, and cursor reflow follow window-copy.c,
 * grid-reader.c, and grid.c at 81f88f8517c9fc5371b56cf117530c6b477c96ac
 * (tmux 3.7). The 3.4 selection rules match at
 * 9ae69c3795ab5ef6b4d760f6398cd9281151f632.
 *
 * Reflow uses captured raw occupied extent, not tmux's historical high-water
 * cellused. Erased-cell high-water behavior and original tabs are not retained.
 *
 * Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
 * Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
 * Copyright (c) 2020 Anindya Mukherjee <anindya49@hotmail.com>
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
#include "pane/copy_mode.hpp"

#include "text/utf8_encode.hpp"

#include <algorithm>
#include <utility>
#include <vector>

namespace tmux_cxx {
namespace {
constexpr std::size_t maximum_backing_rows = 1000U * 512U + 16384U + 512U;
///< Captured cells plus visible padding bound the worst one-column reflow.

/** @brief Require the terminal component's existing positive display-dimension bounds. */
bool valid_copy_size(terminal::CellSize size) {
    return size.rows != 0 && size.columns != 0 && size.rows <= 512 && size.columns <= 512 &&
           static_cast<std::size_t>(size.rows) * size.columns <= 16384;
}

/** @brief Create an unoccupied display cell without borrowing parser storage or rendition. */
terminal::TerminalCell blank_copy_cell() {
    terminal::TerminalCell cell{};
    cell.text = U" ";
    cell.width = 1;
    return cell;
}

/** @brief Count UTF-8 bytes using the encoder's replacement rule for invalid Unicode scalars. */
std::size_t scalar_byte_count(char32_t scalar) {
    if (scalar > 0x10ffff || (scalar >= 0xd800 && scalar <= 0xdfff)) {
        return 3;
    }
    return scalar < 0x80 ? 1 : scalar < 0x800 ? 2 : scalar < 0x10000 ? 3 : 4;
}

/** @brief Reject selection bytes beyond the caller's capacity before appending another scalar. */
bool append_copy_scalar(std::string &bytes, char32_t scalar, std::size_t byte_limit) {
    const auto count = scalar_byte_count(scalar);
    if (count > byte_limit || bytes.size() > byte_limit - count) {
        return false;
    }
    text::append_utf8(bytes, scalar);
    return true;
}

/** @brief Mark a copied cell using the entry-time half-open or inclusive selection convention. */
bool selected_copy_cell(CopyPosition cell, CopyPosition anchor, CopyPosition endpoint,
                        WindowModeKeys keys) {
    const auto first = std::min(anchor, endpoint);
    const auto last = std::max(anchor, endpoint);
    return cell >= first && (keys == WindowModeKeys::vi ? cell <= last : cell < last);
}

/** @brief Retain one logical line's occupied cells while preparing independent resize backing. */
struct LogicalCopyLine {
    std::vector<terminal::TerminalCell> cells; ///< Owned occupied cells prepared for repacking.
    bool continuation; ///< A retained first fragment may continue a discarded predecessor.
};
} // namespace

CopyMode::CopyMode(terminal::TerminalHistorySnapshot backing, WindowModeKeys highlight_keys)
    : backing_(std::move(backing)),
      state_{backing_.size,
             {backing_.history_lines + backing_.cursor.row, backing_.cursor.column},
             backing_.history_lines,
             std::nullopt,
             static_cast<std::uint32_t>(backing_.lines.size()),
             backing_.history_lines,
             backing_.revision,
             1,
             highlight_keys,
             backing_.complete,
             backing_.history_gap,
             backing_.alternate_screen} {}

Result<CopyMode> CopyMode::capture(terminal::TerminalHistorySnapshot backing,
                                   WindowModeKeys highlight_keys) {
    if (!valid_copy_size(backing.size) || !valid_window_mode_keys(highlight_keys) ||
        backing.history_lines > 1000 ||
        backing.lines.size() !=
            static_cast<std::size_t>(backing.history_lines) + backing.size.rows ||
        backing.cursor.row >= backing.size.rows || backing.cursor.column >= backing.size.columns ||
        (backing.alternate_screen && (backing.history_lines != 0 || backing.history_gap))) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "invalid copy-mode source geometry or keys"});
    }
    for (const auto &line : backing.lines) {
        if (line.cells.empty() || line.cells.size() > 512 ||
            line.used_columns > line.cells.size()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "invalid copy-mode source row extent"});
        }
        for (const auto &cell : line.cells) {
            if (cell.width > 2 || (cell.width == 0 && !cell.text.empty())) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::invalid_argument, "invalid copy-mode source cell"});
            }
        }
    }
    return CopyMode{std::move(backing), highlight_keys};
}

std::uint32_t CopyMode::line_length(std::uint32_t row) const {
    const auto &cells = backing_.lines[row].cells;
    auto length = std::min<std::size_t>(cells.size(), state_.size.columns);
    while (length != 0 && cells[length - 1].width != 0 && cells[length - 1].text == U" ") {
        --length;
    }
    return static_cast<std::uint32_t>(length);
}

std::uint32_t CopyMode::cursor_limit(std::uint32_t row, WindowModeKeys keys) const {
    const auto length = line_length(row);
    return keys == WindowModeKeys::vi && length != 0 ? length - 1 : length;
}

void CopyMode::reveal_cursor() {
    if (state_.cursor.row < state_.viewport_top) {
        state_.viewport_top = state_.cursor.row;
    } else if (state_.cursor.row >= state_.viewport_top + state_.size.rows) {
        state_.viewport_top = state_.cursor.row - state_.size.rows + 1;
    }
    state_.viewport_top = std::min(state_.viewport_top, state_.retained_rows - state_.size.rows);
}

bool CopyMode::record_change(const CopyModeState &previous) {
    if (state_ == previous) {
        return false;
    }
    ++state_.revision;
    return true;
}

Result<bool> CopyMode::move(CopyCursorDirection direction, std::uint32_t count,
                            WindowModeKeys keys) {
    if (!valid_copy_cursor_direction(direction) || !valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy cursor direction or keys"});
    }
    if (count == 0) {
        return false;
    }
    const auto previous = state_;
    if (direction == CopyCursorDirection::up || direction == CopyCursorDirection::down) {
        for (std::uint32_t step = 0; step < count; ++step) {
            const auto before_step = state_.cursor;
            const auto old_length = line_length(state_.cursor.row);
            if (state_.cursor.column != old_length) {
                preferred_column_ = state_.cursor.column;
                previous_line_length_ = old_length;
            }
            if (direction == CopyCursorDirection::up && state_.cursor.row != 0) {
                --state_.cursor.row;
            } else if (direction == CopyCursorDirection::down &&
                       state_.cursor.row + 1 < state_.retained_rows) {
                ++state_.cursor.row;
            }
            const auto length = line_length(state_.cursor.row);
            state_.cursor.column = preferred_column_;
            if ((state_.cursor.column >= previous_line_length_ && state_.cursor.column != length) ||
                state_.cursor.column > length) {
                state_.cursor.column = cursor_limit(state_.cursor.row, keys);
            }
            state_.cursor.column =
                std::min(state_.cursor.column, cursor_limit(state_.cursor.row, keys));
            if (state_.cursor == before_step) {
                break;
            }
        }
    } else {
        for (std::uint32_t step = 0; step < count; ++step) {
            const auto before_step = state_.cursor;
            if (direction == CopyCursorDirection::left) {
                const auto &cells = backing_.lines[state_.cursor.row].cells;
                while (state_.cursor.column != 0 && state_.cursor.column < cells.size() &&
                       cells[state_.cursor.column].width == 0) {
                    --state_.cursor.column;
                }
                if (state_.cursor.column == 0 && state_.cursor.row != 0) {
                    --state_.cursor.row;
                    state_.cursor.column = cursor_limit(state_.cursor.row, keys);
                } else if (state_.cursor.column != 0) {
                    --state_.cursor.column;
                }
            } else {
                const auto limit = cursor_limit(state_.cursor.row, keys);
                if (state_.cursor.column >= limit && state_.cursor.row + 1 < state_.retained_rows) {
                    ++state_.cursor.row;
                    state_.cursor.column = 0;
                } else if (state_.cursor.column < limit) {
                    ++state_.cursor.column;
                    const auto &cells = backing_.lines[state_.cursor.row].cells;
                    while (state_.cursor.column < limit && cells[state_.cursor.column].width == 0) {
                        ++state_.cursor.column;
                    }
                }
            }
            state_.cursor.column =
                std::min(state_.cursor.column, cursor_limit(state_.cursor.row, keys));
            if (state_.cursor == before_step) {
                break;
            }
        }
    }
    reveal_cursor();
    return record_change(previous);
}

Result<bool> CopyMode::move_to(CopyCursorDestination destination, WindowModeKeys keys) {
    if (!valid_copy_cursor_destination(destination) || !valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy cursor destination or keys"});
    }
    const auto previous = state_;
    switch (destination) {
    case CopyCursorDestination::line_start:
        while (state_.cursor.row != 0 && backing_.lines[state_.cursor.row].continuation) {
            --state_.cursor.row;
        }
        state_.cursor.column = 0;
        break;
    case CopyCursorDestination::line_end:
        while (state_.cursor.row + 1 < state_.retained_rows &&
               backing_.lines[state_.cursor.row + 1].continuation) {
            ++state_.cursor.row;
        }
        state_.cursor.column = cursor_limit(state_.cursor.row, keys);
        break;
    case CopyCursorDestination::history_start:
        state_.cursor = {0, 0};
        state_.viewport_top = 0;
        break;
    case CopyCursorDestination::history_end:
        state_.cursor.row = state_.retained_rows - 1;
        state_.cursor.column = cursor_limit(state_.cursor.row, keys);
        state_.viewport_top = state_.retained_rows - state_.size.rows;
        break;
    }
    reveal_cursor();
    return record_change(previous);
}

Result<bool> CopyMode::scroll(CopyScrollMotion motion, std::uint32_t count, WindowModeKeys keys) {
    if (!valid_copy_scroll_motion(motion) || !valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy scroll motion or keys"});
    }
    if (count == 0) {
        return false;
    }
    const bool upward = motion == CopyScrollMotion::line_up ||
                        motion == CopyScrollMotion::half_page_up ||
                        motion == CopyScrollMotion::page_up;
    const bool line_step =
        motion == CopyScrollMotion::line_up || motion == CopyScrollMotion::line_down;
    const bool half_page =
        motion == CopyScrollMotion::half_page_up || motion == CopyScrollMotion::half_page_down;
    const std::uint32_t amount = line_step || state_.size.rows <= 2 ? 1
                                 : half_page                        ? state_.size.rows / 2
                                                                    : state_.size.rows - 2;
    const auto previous = state_;
    const auto maximum_top = state_.retained_rows - state_.size.rows;
    for (std::uint32_t step = 0; step < count; ++step) {
        const auto before_step = state_;
        const auto old_length = line_length(state_.cursor.row);
        if (state_.cursor.column != old_length) {
            preferred_column_ = state_.cursor.column;
            previous_line_length_ = old_length;
        }
        auto visual_row = state_.cursor.row - state_.viewport_top;
        if (upward) {
            if (!line_step && state_.viewport_top < amount) {
                state_.viewport_top = 0;
                visual_row -= std::min(amount, visual_row);
            } else {
                state_.viewport_top -= std::min(amount, state_.viewport_top);
            }
        } else if (!line_step && maximum_top - state_.viewport_top < amount) {
            state_.viewport_top = maximum_top;
            visual_row +=
                std::min(amount, static_cast<std::uint32_t>(state_.size.rows - 1) - visual_row);
        } else {
            state_.viewport_top += std::min(amount, maximum_top - state_.viewport_top);
        }
        state_.cursor.row = state_.viewport_top + visual_row;
        const auto length = line_length(state_.cursor.row);
        state_.cursor.column = preferred_column_;
        if ((state_.cursor.column >= previous_line_length_ && state_.cursor.column != length) ||
            state_.cursor.column > length) {
            state_.cursor.column = cursor_limit(state_.cursor.row, keys);
        }
        state_.cursor.column =
            std::min(state_.cursor.column, cursor_limit(state_.cursor.row, keys));
        if (state_ == before_step) {
            break;
        }
    }
    return record_change(previous);
}

bool CopyMode::begin_selection() {
    const auto previous = state_;
    state_.selection_anchor = state_.cursor;
    return record_change(previous);
}

Result<bool> CopyMode::clear_selection(WindowModeKeys keys) {
    if (!valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy selection keys"});
    }
    const auto previous = state_;
    state_.selection_anchor.reset();
    state_.cursor.column = std::min(state_.cursor.column, cursor_limit(state_.cursor.row, keys));
    return record_change(previous);
}

Result<std::string> CopyMode::selected_text(WindowModeKeys keys, std::size_t byte_limit) const {
    if (!valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy selection keys"});
    }
    std::string bytes;
    if (!state_.selection_anchor) {
        return bytes;
    }
    const auto first = std::min(*state_.selection_anchor, state_.cursor);
    const auto last = std::max(*state_.selection_anchor, state_.cursor);
    const auto final_length = line_length(last.row);
    const auto final_end =
        std::min(last.column, final_length) + (keys == WindowModeKeys::vi ? 1 : 0);
    for (auto row = first.row; row <= last.row; ++row) {
        const auto &line = backing_.lines[row];
        const bool continues =
            row + 1 < state_.retained_rows && backing_.lines[row + 1].continuation;
        const bool wrapped = continues && line.used_columns <= state_.size.columns;
        const auto extent = wrapped ? line.used_columns : line_length(row);
        const auto begin = std::min(row == first.row ? first.column : 0U, extent);
        const auto end = std::min(row == last.row ? final_end : state_.size.columns, extent);
        for (auto column = begin; column < end; ++column) {
            const auto &cell = line.cells[column];
            if (cell.width == 0) {
                continue;
            }
            for (const auto scalar : cell.text) {
                if (!append_copy_scalar(bytes, scalar, byte_limit)) {
                    return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                                     "copy selection exceeds byte capacity"});
                }
            }
        }
        const bool remove_final_newline =
            row == last.row && (keys == WindowModeKeys::emacs || final_end <= final_length) &&
            (!continues || final_end != final_length);
        if ((!wrapped || end != extent) && !remove_final_newline &&
            !append_copy_scalar(bytes, U'\n', byte_limit)) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "copy selection exceeds byte capacity"});
        }
    }
    return bytes;
}

Result<CopyMode> CopyMode::resized(terminal::CellSize size, WindowModeKeys keys) const {
    if (!valid_copy_size(size) || !valid_window_mode_keys(keys)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy-mode display size or keys"});
    }
    CopyMode candidate = *this;
    if (size == state_.size) {
        return candidate;
    }
    auto rows = std::move(candidate.backing_.lines);
    auto mapped_cursor = state_.cursor;
    if (size.columns != state_.size.columns) {
        rows.clear();
        std::vector<LogicalCopyLine> logical_lines;
        std::size_t cursor_line = 0;
        std::size_t cursor_offset = 0;
        bool cursor_at_end = false;
        for (std::size_t row = 0; row < backing_.lines.size(); ++row) {
            const auto &line = backing_.lines[row];
            if (row == 0 || !line.continuation) {
                logical_lines.push_back({{}, row == 0 && line.continuation});
            }
            auto &logical = logical_lines.back().cells;
            if (row == state_.cursor.row) {
                cursor_line = logical_lines.size() - 1;
                cursor_offset = logical.size() + state_.cursor.column;
                cursor_at_end = state_.cursor.column >= line.used_columns;
            }
            logical.insert(logical.end(), line.cells.begin(),
                           line.cells.begin() + line.used_columns);
        }
        for (std::size_t logical_index = 0; logical_index < logical_lines.size(); ++logical_index) {
            const auto &logical = logical_lines[logical_index];
            const auto first_row = rows.size();
            rows.push_back({{}, logical.continuation, 0});
            for (std::size_t column = 0; column < logical.cells.size(); ++column) {
                const auto &cell = logical.cells[column];
                if (cell.width == 0) {
                    continue;
                }
                if (rows.back().used_columns != 0 &&
                    rows.back().used_columns + cell.width > size.columns) {
                    rows.push_back({{}, true, 0});
                }
                rows.back().cells.push_back(cell);
                if (cell.width == 2) {
                    auto padding = cell;
                    padding.text.clear();
                    padding.width = 0;
                    rows.back().cells.push_back(std::move(padding));
                }
                rows.back().used_columns += cell.width;
            }
            if (logical_index == cursor_line) {
                if (cursor_at_end) {
                    mapped_cursor = {static_cast<std::uint32_t>(rows.size() - 1),
                                     rows.back().used_columns};
                } else {
                    auto remaining = cursor_offset;
                    auto row = first_row;
                    while (row + 1 < rows.size() && remaining >= rows[row].used_columns) {
                        remaining -= rows[row].used_columns;
                        ++row;
                    }
                    mapped_cursor = {static_cast<std::uint32_t>(row),
                                     static_cast<std::uint32_t>(remaining)};
                }
            }
        }
    }
    if (rows.size() > maximum_backing_rows) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "copy-mode reflow exceeds retained-row capacity"});
    }
    while (rows.size() < size.rows) {
        rows.push_back({{}, false, 0});
    }
    candidate.backing_.lines = std::move(rows);
    candidate.backing_.size = size;
    candidate.state_.size = size;
    candidate.state_.cursor = mapped_cursor;
    candidate.state_.retained_rows = static_cast<std::uint32_t>(candidate.backing_.lines.size());
    candidate.state_.viewport_top =
        std::min(mapped_cursor.row, candidate.state_.retained_rows - size.rows);
    candidate.state_.selection_anchor.reset();
    candidate.state_.cursor.column =
        std::min(candidate.state_.cursor.column, candidate.cursor_limit(mapped_cursor.row, keys));
    ++candidate.state_.revision;
    return candidate;
}

PaneScreenSnapshot CopyMode::screen_snapshot() const {
    PaneScreenSnapshot snapshot{
        state_.size,
        {static_cast<std::uint16_t>(state_.cursor.row - state_.viewport_top),
         static_cast<std::uint16_t>(
             std::min<std::uint32_t>(state_.cursor.column, state_.size.columns - 1))},
        true,
        state_.complete,
        state_.revision,
        {}};
    snapshot.cells.reserve(static_cast<std::size_t>(state_.size.rows) * state_.size.columns);
    const auto blank = blank_copy_cell();
    for (std::uint32_t visual_row = 0; visual_row < state_.size.rows; ++visual_row) {
        const auto backing_row = state_.viewport_top + visual_row;
        const auto &line = backing_.lines[backing_row];
        for (std::uint32_t column = 0; column < state_.size.columns; ++column) {
            auto cell = column < line.cells.size() ? line.cells[column] : blank;
            if (cell.width == 2 && column + 2 > state_.size.columns) {
                // A one-column view hides an unfit glyph; backing retains it for later growth.
                cell = blank;
            }
            if (state_.selection_anchor &&
                selected_copy_cell({backing_row, column}, *state_.selection_anchor, state_.cursor,
                                   state_.highlight_keys)) {
                cell.reverse = !cell.reverse;
            }
            snapshot.cells.push_back(std::move(cell));
        }
    }
    return snapshot;
}
} // namespace tmux_cxx
