#pragma once

#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/session/session_observation.hpp>
#include <tmux_cxx/tmux_error.hpp>

namespace tmux_cxx {
class Server;
/** @brief Read owner-qualified committed events without exposing the server's journal storage. */
class SessionEventReader {
  public:
    /** @brief Materialize sessions and the following event cursor at one server-thread boundary. */
    static SessionObservation observe_sessions(const Server &server);
    /** @brief Establish a no-replay cursor after every event committed so far. */
    static SessionEventCursor tail(const Server &server);
    /** @brief Copy retained events or reject a cursor from another server or discarded prefix. */
    static Result<SessionEventBatch> read(const Server &server, const SessionEventCursor &cursor);
};
} // namespace tmux_cxx
