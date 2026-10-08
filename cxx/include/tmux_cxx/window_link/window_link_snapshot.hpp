#pragma once

#include <tmux_cxx/identity.hpp>

namespace tmux_cxx {
/** @brief Materialized membership assigning a shared window an index within one session. */
struct WindowLinkSnapshot {
    /** @brief Stable membership identity within the observed server. */
    WindowLinkId id;
    /** @brief Session owning this membership and its index. */
    SessionId session;
    /** @brief Shared window retained while any membership exists. */
    WindowId window;
    /** @brief Window index within this membership's session. */
    std::uint32_t index;
    /** @brief Server mutation revision establishing this membership. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
