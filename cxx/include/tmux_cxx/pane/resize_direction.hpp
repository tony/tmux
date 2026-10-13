#pragma once

#include <cstdint>

namespace tmux_cxx {
/** @brief Choose a border movement; the final pane uses its preceding border.
 *
 * Fixed storage permits rejecting invalid values before native wire narrowing. */
enum class PaneResizeDirection : std::uint32_t {
    left,  ///< Move a vertical border left.
    right, ///< Move a vertical border right.
    up,    ///< Move a horizontal border up.
    down,  ///< Move a horizontal border down.
};
} // namespace tmux_cxx
