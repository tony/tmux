#pragma once

namespace tmux_cxx {
/** @brief Choose the ordered arrangement created by splitting one pane. */
enum class PaneSplitOrientation {
    left_right, ///< Keep the target on the left and place the new pane on the right.
    top_bottom, ///< Keep the target above and place the new pane below.
};
} // namespace tmux_cxx
