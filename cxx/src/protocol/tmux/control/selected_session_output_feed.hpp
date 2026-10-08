#pragma once

#include "protocol/tmux/control/channel.hpp"

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/pane/pane_output_cursor.hpp>

#include <map>
#include <optional>

namespace tmux_cxx {
class Server;
namespace protocol::tmux::control {
/** @brief Track one control client's selected session and queued raw pane-output cursors. */
class SelectedSessionOutputFeed {
  public:
    /** @brief Queue session changes and newly retained pane bytes without replaying old output. */
    Result<void> queue_new_output(Server &server, const ClientSnapshot &client, Channel &channel);

  private:
    std::optional<SessionId> selected_session_;       ///< Session whose pane set owns the cursors.
    std::map<PaneId, PaneOutputCursor> pane_cursors_; ///< Per-pane queued-output positions.
};
} // namespace protocol::tmux::control
} // namespace tmux_cxx
