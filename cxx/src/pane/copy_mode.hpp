#pragma once

#include <tmux_cxx/pane/copy_cursor_destination.hpp>
#include <tmux_cxx/pane/copy_cursor_direction.hpp>
#include <tmux_cxx/pane/copy_position.hpp>
#include <tmux_cxx/pane/copy_scroll_motion.hpp>
#include <tmux_cxx/pane/pane_screen_snapshot.hpp>
#include <tmux_cxx/terminal/terminal_history_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>
#include <tmux_cxx/window/mode_keys.hpp>

#include <cstddef>
#include <optional>
#include <string>

namespace tmux_cxx {
/** @brief Retain copy-mode coordinates, frozen-source evidence, and independent display freshness.
 */
struct CopyModeState {
    terminal::CellSize size;                      ///< Current mode display dimensions.
    CopyPosition cursor;                          ///< Absolute cursor within frozen backing.
    std::uint32_t viewport_top;                   ///< Absolute first visible backing row.
    std::optional<CopyPosition> selection_anchor; ///< Anchor whose endpoint is the current cursor.
    std::uint32_t retained_rows;                  ///< Current backing row count after any reflow.
    std::uint32_t source_history_lines;           ///< Retained primary-history rows at entry.
    std::uint64_t source_revision;                ///< Program parser revision captured on entry.
    std::uint64_t revision;                       ///< Mode display sequence independent of parsing.
    WindowModeKeys highlight_keys; ///< Selection-highlight convention captured on entry.
    bool complete;                 ///< Completeness of the captured parser state.
    bool history_gap;              ///< Captured evidence of a discarded history prefix.
    bool alternate_screen;         ///< Whether entry captured the active alternate screen.
    /** @brief Compare copied mode metadata without retaining backing-row references. */
    bool operator==(const CopyModeState &) const = default;
};

/** @brief Own frozen terminal rows, character selection, navigation, and independent resize
 * candidates. */
class CopyMode {
  public:
    /** @brief Validate and own chronological backing while caching entry-time highlight keys. */
    static Result<CopyMode> capture(terminal::TerminalHistorySnapshot backing,
                                    WindowModeKeys highlight_keys);
    /** @brief Move a counted relative cursor using the current window's key convention. */
    Result<bool> move(CopyCursorDirection direction, std::uint32_t count, WindowModeKeys keys);
    /** @brief Move directly to an uncounted logical-line or history boundary. */
    Result<bool> move_to(CopyCursorDestination destination, WindowModeKeys keys);
    /** @brief Scroll counted line, half-page, or page steps using current cursor conventions. */
    Result<bool> scroll(CopyScrollMotion motion, std::uint32_t count, WindowModeKeys keys);
    /** @brief Start character selection at the current frozen-backing cursor. */
    bool begin_selection();
    /** @brief Clear selection and clip its cursor using the current window's key convention. */
    Result<bool> clear_selection(WindowModeKeys keys);
    /** @brief Extract bounded UTF-8 selection bytes without changing mode state. */
    Result<std::string> selected_text(WindowModeKeys keys, std::size_t byte_limit) const;
    /** @brief Prepare resized backing and current-key cursor without changing this mode. */
    Result<CopyMode> resized(terminal::CellSize size, WindowModeKeys keys) const;
    /** @brief Copy the visible frozen display and its entry-style selection highlighting. */
    PaneScreenSnapshot screen_snapshot() const;
    /** @brief Borrow mode metadata until the next mode mutation or destruction. */
    const CopyModeState &state() const { return state_; }

  private:
    /** @brief Initialize owned backing after capture has validated its dimensions and cells. */
    CopyMode(terminal::TerminalHistorySnapshot backing, WindowModeKeys highlight_keys);
    /** @brief Measure the visible row extent after source-defined trailing-space trimming. */
    std::uint32_t line_length(std::uint32_t row) const;
    /** @brief Resolve the current key-style cursor limit for one backing row. */
    std::uint32_t cursor_limit(std::uint32_t row, WindowModeKeys keys) const;
    /** @brief Keep the absolute cursor inside the visible mode viewport. */
    void reveal_cursor();
    /** @brief Record a visible state change while preserving the captured source revision. */
    bool record_change(const CopyModeState &previous);
    terminal::TerminalHistorySnapshot backing_; ///< Owned original-width chronological rows.
    CopyModeState state_;                ///< Cursor, selection, viewport, and source evidence.
    std::uint32_t preferred_column_ = 0; ///< Horizontal goal retained across vertical movement.
    std::uint32_t previous_line_length_ =
        0; ///< Prior row extent used by tmux's end-of-line affinity.
};
} // namespace tmux_cxx
