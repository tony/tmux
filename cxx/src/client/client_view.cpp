#include "client/client_view.hpp"

#include "text/utf8_encode.hpp"

#include <algorithm>
#include <charconv>
#include <cstdint>
#include <span>

namespace tmux_cxx {
namespace {
constexpr std::size_t max_redraw_bytes = 1048576; ///< Per-client full-redraw allocation bound.
constexpr std::size_t max_composed_cells = 512U * 512U; ///< Bounded layout-validation storage.

/** @brief Select the terminal color plane encoded by one SGR parameter. */
enum class ColorPlane { foreground, background };

/** @brief Append one decimal terminal parameter without locale or heap allocation. */
void append_decimal(std::string &redraw, std::uint32_t value) {
    char digits[10]{};
    const auto converted = std::to_chars(std::begin(digits), std::end(digits), value);
    redraw.append(digits, converted.ptr);
}

/** @brief Append one SGR parameter with its required separator. */
void append_parameter(std::string &redraw, std::string_view parameter) {
    redraw.push_back(';');
    redraw.append(parameter);
}

/** @brief Append a foreground or background color while retaining indexed and RGB meaning. */
void append_color(std::string &redraw, const terminal::TerminalColor &color, ColorPlane plane) {
    if (color.kind == terminal::TerminalColorKind::default_color) {
        return;
    }
    append_parameter(redraw, plane == ColorPlane::foreground ? "38" : "48");
    if (color.kind == terminal::TerminalColorKind::indexed) {
        append_parameter(redraw, "5");
        append_decimal(redraw, color.value);
        return;
    }
    append_parameter(redraw, "2");
    append_decimal(redraw, (color.value >> 16) & 0xffU);
    redraw.push_back(';');
    append_decimal(redraw, (color.value >> 8) & 0xffU);
    redraw.push_back(';');
    append_decimal(redraw, color.value & 0xffU);
}

/** @brief Reset rendition and encode the next cell's supported terminal attributes. */
void append_style(std::string &redraw, const terminal::TerminalCell &cell) {
    redraw.append("\x1b[0");
    if (cell.bold) {
        append_parameter(redraw, "1");
    }
    if (cell.italic) {
        append_parameter(redraw, "3");
    }
    if (cell.underline == 1) {
        append_parameter(redraw, "4");
    } else if (cell.underline == 2) {
        append_parameter(redraw, "21");
    } else if (cell.underline == 3) {
        append_parameter(redraw, "4:3");
    }
    if (cell.blink) {
        append_parameter(redraw, "5");
    }
    if (cell.reverse) {
        append_parameter(redraw, "7");
    }
    if (cell.conceal) {
        append_parameter(redraw, "8");
    }
    if (cell.strike) {
        append_parameter(redraw, "9");
    }
    if (cell.font > 0) {
        append_parameter(redraw, std::to_string(10U + cell.font));
    }
    append_color(redraw, cell.foreground, ColorPlane::foreground);
    append_color(redraw, cell.background, ColorPlane::background);
    redraw.push_back('m');
}

/** @brief Compare client-rendered attributes while leaving line-size metadata to future layout. */
bool same_style(const terminal::TerminalCell &left, const terminal::TerminalCell &right) {
    return left.foreground == right.foreground && left.background == right.background &&
           left.bold == right.bold && left.italic == right.italic && left.blink == right.blink &&
           left.reverse == right.reverse && left.conceal == right.conceal &&
           left.strike == right.strike && left.underline == right.underline &&
           left.font == right.font;
}

/** @brief Move to one zero-based window cell using a one-based ANSI cursor address. */
void append_cursor_position(std::string &redraw, std::uint32_t row, std::uint32_t column) {
    redraw.append("\x1b[");
    append_decimal(redraw, row + 1);
    redraw.push_back(';');
    append_decimal(redraw, column + 1);
    redraw.push_back('H');
}

/** @brief Reject malformed copied layout state before emitting terminal effects. */
Result<std::vector<std::uint8_t>>
validate_window_view(const ClientWindowViewSnapshot &window_view) {
    const auto rows = static_cast<std::size_t>(window_view.size.rows);
    const auto columns = static_cast<std::size_t>(window_view.size.columns);
    if (rows == 0 || columns == 0 || rows > max_composed_cells / columns ||
        rows * columns > max_composed_cells || window_view.panes.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "client window view has invalid dimensions"});
    }
    std::vector<std::uint8_t> occupied(rows * columns, 0);
    bool active_pane_found = false;
    for (std::size_t pane_index = 0; pane_index < window_view.panes.size(); ++pane_index) {
        const auto &pane = window_view.panes[pane_index];
        const auto &geometry = pane.geometry;
        if (!pane.terminal.complete || geometry.size.rows == 0 || geometry.size.columns == 0 ||
            geometry.size.rows > window_view.size.rows ||
            geometry.size.columns > window_view.size.columns ||
            geometry.top > static_cast<std::uint32_t>(window_view.size.rows - geometry.size.rows) ||
            geometry.left >
                static_cast<std::uint32_t>(window_view.size.columns - geometry.size.columns) ||
            pane.terminal.size != geometry.size ||
            pane.terminal.cells.size() !=
                static_cast<std::size_t>(geometry.size.rows) * geometry.size.columns) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "client pane view is incomplete or misplaced"});
        }
        for (std::size_t prior = 0; prior < pane_index; ++prior) {
            if (window_view.panes[prior].geometry.pane == geometry.pane) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::protocol, "client window view repeats a pane"});
            }
        }
        if (pane.active) {
            if (active_pane_found) {
                return std::unexpected(TmuxError{TmuxErrorCode::protocol,
                                                 "client window view has multiple active panes"});
            }
            active_pane_found = true;
        }
        for (std::uint32_t row = 0; row < geometry.size.rows; ++row) {
            for (std::uint32_t column = 0; column < geometry.size.columns; ++column) {
                const auto position =
                    static_cast<std::size_t>(geometry.top + row) * columns + geometry.left + column;
                if (occupied[position] != 0) {
                    return std::unexpected(
                        TmuxError{TmuxErrorCode::protocol, "client pane views overlap"});
                }
                occupied[position] = 1;
            }
        }
    }
    if (!active_pane_found) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "client window view has no active pane"});
    }
    return occupied;
}

/** @brief Draw one copied pane interior at its clipped window coordinates. */
Result<void> append_pane_view(std::string &redraw, const ClientPaneViewSnapshot &pane,
                              terminal::CellSize viewport) {
    if (pane.geometry.top >= viewport.rows || pane.geometry.left >= viewport.columns) {
        return {};
    }
    const auto rows = std::min<std::uint32_t>(
        pane.geometry.size.rows, static_cast<std::uint32_t>(viewport.rows) - pane.geometry.top);
    const auto columns =
        std::min<std::uint32_t>(pane.geometry.size.columns,
                                static_cast<std::uint32_t>(viewport.columns) - pane.geometry.left);
    for (std::uint32_t row = 0; row < rows; ++row) {
        append_cursor_position(redraw, pane.geometry.top + row, pane.geometry.left);
        const terminal::TerminalCell *rendered_style = nullptr;
        for (std::uint32_t column = 0; column < columns; ++column) {
            const auto &cell = pane.terminal.cells[row * pane.terminal.size.columns + column];
            if (cell.width == 0) {
                continue;
            }
            if (rendered_style == nullptr || !same_style(*rendered_style, cell)) {
                append_style(redraw, cell);
                rendered_style = &cell;
            }
            if (cell.width > 1 && column + cell.width > columns) {
                redraw.push_back(' ');
                continue;
            }
            if (cell.text.empty()) {
                redraw.push_back(' ');
            } else {
                for (const auto scalar : cell.text) {
                    text::append_utf8(redraw, scalar);
                }
            }
        }
        redraw.append("\x1b[0m");
        if (redraw.size() > max_redraw_bytes) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure, "composed client view exceeds its bound"});
        }
    }
    return {};
}

/** @brief Select an ASCII separator glyph from adjacent internal layout-border cells. */
char border_glyph(std::span<const std::uint8_t> occupied, terminal::CellSize layout_size,
                  std::uint32_t row, std::uint32_t column) {
    const auto columns = static_cast<std::size_t>(layout_size.columns);
    /** @brief Test whether one in-bounds neighbor belongs to the layout border. */
    const auto is_border = [&](std::uint32_t candidate_row, std::uint32_t candidate_column) {
        return occupied[static_cast<std::size_t>(candidate_row) * columns + candidate_column] == 0;
    };
    const bool left = column > 0 && is_border(row, column - 1);
    const bool right = column + 1 < layout_size.columns && is_border(row, column + 1);
    const bool above = row > 0 && is_border(row - 1, column);
    const bool below = row + 1 < layout_size.rows && is_border(row + 1, column);
    if ((left || right) && (above || below)) {
        return '+';
    }
    if (left || right) {
        return '-';
    }
    if (above || below) {
        return '|';
    }
    return '+';
}

/** @brief Draw all clipped separator cells left by the validated pane partition. */
Result<void> append_layout_borders(std::string &redraw, std::span<const std::uint8_t> occupied,
                                   terminal::CellSize layout_size, terminal::CellSize viewport) {
    const auto rows = std::min(layout_size.rows, viewport.rows);
    const auto columns = std::min(layout_size.columns, viewport.columns);
    for (std::uint32_t row = 0; row < rows; ++row) {
        std::uint32_t column = 0;
        while (column < columns) {
            const auto position = static_cast<std::size_t>(row) * layout_size.columns + column;
            if (occupied[position] != 0) {
                ++column;
                continue;
            }
            append_cursor_position(redraw, row, column);
            redraw.append("\x1b[0m");
            do {
                redraw.push_back(border_glyph(occupied, layout_size, row, column));
                ++column;
            } while (column < columns &&
                     occupied[static_cast<std::size_t>(row) * layout_size.columns + column] == 0);
        }
        if (redraw.size() > max_redraw_bytes) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure, "composed client view exceeds its bound"});
        }
    }
    return {};
}

/** @brief Restore the visible cursor only for the active pane inside the client viewport. */
void append_active_pane_cursor(std::string &redraw, const ClientWindowViewSnapshot &window_view,
                               terminal::CellSize viewport) {
    const auto active = std::ranges::find(window_view.panes, true, &ClientPaneViewSnapshot::active);
    if (active == window_view.panes.end() || !active->terminal.cursor_visible ||
        active->terminal.cursor.row >= active->geometry.size.rows ||
        active->terminal.cursor.column >= active->geometry.size.columns) {
        return;
    }
    const auto row = active->geometry.top + active->terminal.cursor.row;
    const auto column = active->geometry.left + active->terminal.cursor.column;
    if (row >= viewport.rows || column >= viewport.columns) {
        return;
    }
    append_cursor_position(redraw, row, column);
    redraw.append("\x1b[?25h");
}
} // namespace

bool ClientView::needs_redraw(const ClientWindowViewSnapshot &window_view,
                              terminal::CellSize viewport) const {
    if (!screen_entry_queued_ || queued_window_ != window_view.window ||
        queued_window_revision_ != window_view.window_revision || queued_viewport_ != viewport ||
        queued_pane_revisions_.size() != window_view.panes.size()) {
        return true;
    }
    for (std::size_t index = 0; index < window_view.panes.size(); ++index) {
        if (!window_view.panes[index].terminal.complete ||
            queued_pane_revisions_[index].pane != window_view.panes[index].geometry.pane ||
            queued_pane_revisions_[index].terminal_revision !=
                window_view.panes[index].terminal.revision) {
            return true;
        }
    }
    return false;
}

Result<std::string> ClientView::compose_redraw(const ClientWindowViewSnapshot &window_view,
                                               terminal::CellSize viewport) const {
    if (viewport.rows == 0 || viewport.columns == 0) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "cannot compose an incomplete client view"});
    }
    auto occupied = validate_window_view(window_view);
    if (!occupied) {
        return std::unexpected(occupied.error());
    }
    std::string redraw;
    redraw.reserve(std::min<std::size_t>(
        max_redraw_bytes, static_cast<std::size_t>(viewport.rows) * viewport.columns * 8U + 64U));
    if (!screen_entry_queued_) {
        redraw.append("\x1b[?1049h");
    }
    redraw.append("\x1b[?25l\x1b[H\x1b[2J");
    for (const auto &pane : window_view.panes) {
        if (auto appended = append_pane_view(redraw, pane, viewport); !appended) {
            return std::unexpected(appended.error());
        }
    }
    if (auto appended = append_layout_borders(redraw, *occupied, window_view.size, viewport);
        !appended) {
        return std::unexpected(appended.error());
    }
    append_active_pane_cursor(redraw, window_view, viewport);
    if (redraw.size() > max_redraw_bytes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "composed client view exceeds its bound"});
    }
    return redraw;
}

void ClientView::record_queued_redraw(const ClientWindowViewSnapshot &window_view,
                                      terminal::CellSize viewport) {
    queued_window_ = window_view.window;
    queued_window_revision_ = window_view.window_revision;
    queued_pane_revisions_.clear();
    queued_pane_revisions_.reserve(window_view.panes.size());
    for (const auto &pane : window_view.panes) {
        queued_pane_revisions_.push_back({pane.geometry.pane, pane.terminal.revision});
    }
    queued_viewport_ = viewport;
    screen_entry_queued_ = true;
}

void ClientView::invalidate() {
    queued_window_ = {};
    queued_window_revision_ = 0;
    queued_pane_revisions_.clear();
    queued_viewport_ = {};
    screen_entry_queued_ = false;
}
} // namespace tmux_cxx
