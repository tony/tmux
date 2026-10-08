#pragma once

#include <tmux_cxx/pane/pane_output_cursor.hpp>
#include <tmux_cxx/tmux_error.hpp>

namespace tmux_cxx {
class Server;

/** @brief Resolve owner-qualified raw pane-output cursors against current server state. */
class PaneOutputReader {
  public:
    /** @brief Capture the current stream tail without replaying previously retained bytes. */
    static Result<PaneOutputCursor> tail(const Server &server, PaneId pane);
    /** @brief Copy retained bytes after a cursor or report wrong ownership, deletion, or a gap. */
    static Result<PaneOutputChunk> read(const Server &server, const PaneOutputCursor &cursor);
};
} // namespace tmux_cxx
