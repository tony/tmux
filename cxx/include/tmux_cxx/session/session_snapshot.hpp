#pragma once

#include <tmux_cxx/identity.hpp>

#include <string>

namespace tmux_cxx {
/** @brief A session value that survives deletion and connection disposal. */
struct SessionSnapshot {
    /** @brief Stable session identity within the observed server instance. */
    SessionId id;
    /** @brief Window selected through this session's current link. */
    WindowId window;
    /** @brief Session membership connecting the selected window to this session. */
    WindowLinkId link;
    /** @brief Selected pane within the linked window. */
    PaneId pane;
    /** @brief Session name at this observation revision. */
    std::string name;
    /** @brief Server mutation revision of this session's most recent change. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
