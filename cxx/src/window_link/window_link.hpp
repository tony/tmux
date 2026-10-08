#pragma once

#include <tmux_cxx/identity.hpp>

namespace tmux_cxx {
/** @brief Connect one session index to a shared window without transferring window ownership. */
struct WindowLink {
    WindowLinkId id;        ///< Stable membership identity.
    SessionId session;      ///< Session owning this membership's index.
    WindowId window;        ///< Shared window retained by this membership.
    std::uint32_t index;    ///< Window index scoped to the owning session.
    std::uint64_t revision; ///< Revision at which this membership was established.
};
} // namespace tmux_cxx
