#pragma once

#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

#include <vector>

namespace tmux_cxx {
/** @brief Pair materialized sessions with the first committed session event after their snapshot.
 */
struct SessionObservation {
    /** @brief Sessions materialized at one server-thread boundary. */
    std::vector<SessionSnapshot> sessions;
    /** @brief First event committed after that snapshot boundary. */
    SessionEventCursor next_event;
};
} // namespace tmux_cxx
