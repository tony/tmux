#pragma once

#include <string>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Format one session-catalog mutation as a stock control notification. */
std::string format_sessions_changed();
} // namespace tmux_cxx::protocol::tmux::control
