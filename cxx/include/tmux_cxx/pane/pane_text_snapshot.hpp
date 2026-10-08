#pragma once

#include <tmux_cxx/identity.hpp>

#include <string>
#include <vector>

namespace tmux_cxx {
/** @brief Own interpreted pane text and freshness independently of raw bytes and client rendering.
 */
struct PaneTextSnapshot {
    /** @brief Server instance owning this text observation. */
    std::string server_instance;
    /** @brief Pane identity within the observed server instance. */
    PaneId pane;
    /** @brief Terminal parser/resize sequence, distinct from server metadata revisions. */
    std::uint64_t revision;
    /** @brief Program terminal height in character cells. */
    std::uint32_t rows;
    /** @brief Program terminal width in character cells. */
    std::uint32_t columns;
    /** @brief Zero-based cursor row within the active screen. */
    std::uint32_t cursor_row;
    /** @brief Zero-based cursor column within the active screen. */
    std::uint32_t cursor_column;
    /** @brief Requested terminal cursor visibility. */
    bool cursor_visible;
    /** @brief The alternate screen is active. */
    bool alternate_screen;
    /** @brief Every consumed chunk completed without a contained terminal callback failure. */
    bool complete;
    /** @brief Owned UTF-8 rows; wide-cell continuations add no extra text or spaces. */
    std::vector<std::string> lines;
    /** @brief Number of retained primary terminal-history rows. */
    std::uint32_t history_lines;
    /** @brief An earlier terminal-history prefix was discarded; this cannot recover raw bytes. */
    bool history_gap;
    /** @brief The PTY and program-input queue still accept input. */
    bool input_open;
    /** @brief Accepted input or terminal replies still await PTY delivery. */
    bool input_pending;
};
} // namespace tmux_cxx
