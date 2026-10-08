#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

namespace tmux_cxx {
class Server;
/** @brief Name the policy selecting shared window dimensions from attached clients. */
enum class WindowSizePolicy {
    latest ///< The most recently attached or resized client selects shared dimensions.
};

/** @brief Apply tmux's latest-client size policy to a session's selected shared window. */
class LatestClientWindowSize {
  public:
    /** @brief Resize the selected window layout, every pane terminal, and affected client views. */
    static Result<void> apply(Server &server, SessionId session_id, terminal::CellSize size);
};
} // namespace tmux_cxx
