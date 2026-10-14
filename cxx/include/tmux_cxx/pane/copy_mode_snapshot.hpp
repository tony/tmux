#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/copy_position.hpp>
#include <tmux_cxx/window/mode_keys.hpp>

#include <optional>
#include <string>
#include <vector>

namespace tmux_cxx {
/** @brief Copy an explicit character selection in frozen backing coordinates. */
struct CopySelectionSnapshot {
    /** @brief Absolute backing cell where character selection began. */
    CopyPosition anchor;
    /** @brief Current cursor endpoint; extraction inclusion follows the live window key convention.
     */
    CopyPosition end;
};

/** @brief Own the visible copy viewport, selection, and frozen-source evidence. */
struct CopyModeStateSnapshot {
    /** @brief Current copy viewport height in character cells. */
    std::uint32_t rows;
    /** @brief Current copy viewport width in character cells. */
    std::uint32_t columns;
    /** @brief Retained backing row count after any copy-owned reflow. */
    std::uint32_t backing_lines;
    /** @brief Absolute backing row displayed at the viewport's top. */
    std::uint32_t viewport_top;
    /** @brief Absolute frozen-backing cursor, including a permitted end-of-line position. */
    CopyPosition cursor;
    /** @brief Explicit character selection, absent until selection begins. */
    std::optional<CopySelectionSnapshot> selection;
    /** @brief Program parser revision captured on entry; later output does not change it. */
    std::uint64_t source_revision;
    /** @brief Primary terminal-history rows captured on entry, before any copy-owned reflow. */
    std::uint32_t source_history_lines;
    /** @brief Entry captured the alternate screen without hidden primary history. */
    bool alternate_screen;
    /** @brief The captured terminal state completed without a contained parser failure. */
    bool complete;
    /** @brief A history prefix had already been discarded before entry. */
    bool history_gap;
    /** @brief Live inherited window convention for input, movement, and extraction. */
    WindowModeKeys mode_keys;
    /** @brief Selection-highlight convention captured when this mode was entered. */
    WindowModeKeys highlight_keys;
    /** @brief Owned UTF-8 visible rows; wide-cell continuations add no duplicate text. */
    std::vector<std::string> lines;
};

/** @brief Observe one pane's copy mode without borrowing server or backing storage. */
struct CopyModeSnapshot {
    /** @brief Server instance owning this pane and copy observation. */
    std::string server_instance;
    /** @brief Pane identity within the observed server instance. */
    PaneId pane;
    /** @brief Freshness of the pane's chosen display, independent of program-parser revisions. */
    std::uint64_t display_revision;
    /** @brief Frozen mode state, absent while the pane displays its program screen. */
    std::optional<CopyModeStateSnapshot> mode;
};
} // namespace tmux_cxx
