#pragma once

#include <tmux_cxx/identity.hpp>

namespace tmux_cxx {
/** @brief Materialize one pane's window ownership, layout geometry, and selection state. */
struct PaneSnapshot {
    /** @brief Stable pane identity within the observed server. */
    PaneId id;
    /** @brief Shared window that exclusively owns this pane. */
    WindowId window;
    /** @brief Zero-based top row within the shared window. */
    std::uint32_t top;
    /** @brief Zero-based left column within the shared window. */
    std::uint32_t left;
    /** @brief Program-terminal height excluding layout borders. */
    std::uint32_t rows;
    /** @brief Program-terminal width excluding layout borders. */
    std::uint32_t columns;
    /** @brief Whether the owning window currently selects this pane. */
    bool active;
    /** @brief Owning window revision that produced this geometry. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
