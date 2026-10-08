#pragma once

#include <tmux_cxx/identity.hpp>

#include <string>

namespace tmux_cxx {
/** @brief Materialized window state shared by all sessions linked to that window. */
struct WindowSnapshot {
    /** @brief Stable window identity within the observed server. */
    WindowId id;
    /** @brief Active pane shared by every link to this window. */
    PaneId pane;
    /** @brief Window name at this revision. */
    std::string name;
    /** @brief Shared program-terminal height selected by attached clients. */
    std::uint32_t rows;
    /** @brief Shared program-terminal width selected by attached clients. */
    std::uint32_t columns;
    /** @brief Server mutation revision of the window's most recent change. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
