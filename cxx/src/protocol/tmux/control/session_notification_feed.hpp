#pragma once

#include "protocol/tmux/control/channel.hpp"

#include <tmux_cxx/session/session_event.hpp>

#include <utility>

namespace tmux_cxx {
class Server;
namespace protocol::tmux::control {
/** @brief Translate committed session events for one control client from an owned cursor. */
class SessionNotificationFeed {
  public:
    /** @brief Start after the supplied event baseline without replaying earlier mutations. */
    explicit SessionNotificationFeed(SessionEventCursor cursor)
        : event_cursor_(std::move(cursor)) {}
    /** @brief Queue every retained session notification in commit order. */
    Result<void> queue_notifications(const Server &server, Channel &channel);

  private:
    SessionEventCursor event_cursor_; ///< Next committed session event this client has not queued.
};
} // namespace protocol::tmux::control
} // namespace tmux_cxx
