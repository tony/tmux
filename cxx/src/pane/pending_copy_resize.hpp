#pragma once

#include "pane/copy_mode.hpp"

#include <tmux_cxx/identity.hpp>

namespace tmux_cxx {
/** @brief Own a reflowed mode until the pane's containing geometry mutation succeeds. */
struct PendingPaneCopyResize {
    PaneId pane;   ///< Stable identity resolved again at commit.
    CopyMode mode; ///< Independent backing, cursor, and selection candidate.
};
} // namespace tmux_cxx
