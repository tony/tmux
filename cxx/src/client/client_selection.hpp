#pragma once

#include <tmux_cxx/identity.hpp>

namespace tmux_cxx {
/** @brief Copy an attached client's current session, membership, window, and pane identities. */
struct ClientSelection {
    SessionId session;        ///< Session to which this client is attached.
    WindowLinkId window_link; ///< Selected membership owned by that session.
    WindowId window;          ///< Shared window reached through the selected membership.
    PaneId pane;              ///< Active pane within the selected window.
};
} // namespace tmux_cxx
