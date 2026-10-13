#pragma once

#include "window/layout.hpp"
#include "window/window_size.hpp"

#include <tmux_cxx/identity.hpp>

#include <set>
#include <string>
#include <vector>

namespace tmux_cxx {
/** @brief Own shared pane membership and selection independently of any linked session. */
struct Window {
    WindowId id;                    ///< Stable identity allocated by this server.
    std::string name;               ///< Shared window name.
    WindowLayout layout;            ///< Partition tree owning geometric pane placement.
    PaneId active;                  ///< Active pane shared by every linked session.
    std::vector<PaneId> pane_order; ///< Logical pane order independent of geometric placement.
    std::set<WindowLinkId> links;   ///< Memberships retaining this window and its panes.
    terminal::CellSize size;        ///< Shared program dimensions selected by the size policy.
    WindowSizePolicy size_policy;   ///< Declared policy for competing attached-client dimensions.
    std::uint64_t revision;         ///< Most recent shared-window mutation.
};
} // namespace tmux_cxx
