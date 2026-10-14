#pragma once

#include <tmux_cxx/terminal/terminal_snapshot.hpp>

namespace tmux_cxx::terminal {
/** @brief Own one retained terminal row and its logical-line boundary. */
struct TerminalLine {
    std::vector<TerminalCell> cells; ///< Copied cells at the row's original width.
    bool continuation;               ///< This row continues the preceding logical line.
    std::uint32_t used_columns =
        0; ///< Raw occupied extent, including written spaces and wide padding.
};

/** @brief Capture chronological retained history followed by the active visible screen. */
struct TerminalHistorySnapshot {
    CellSize size;               ///< Active program terminal dimensions in cells.
    CellPosition cursor;         ///< Cursor within the active visible screen.
    bool cursor_visible;         ///< Program-requested cursor visibility at capture.
    bool alternate_screen;       ///< Alternate captures contain only active visible rows.
    bool complete;               ///< False after a contained parser or callback failure.
    std::uint64_t revision;      ///< Parser/resize sequence at this serialized capture.
    std::uint32_t history_lines; ///< Retained primary rows preceding the visible screen.
    bool history_gap; ///< A primary-history prefix was discarded; false for alternate captures.
    std::vector<TerminalLine> lines; ///< Owned chronological rows with independent cell storage.
};
} // namespace tmux_cxx::terminal
