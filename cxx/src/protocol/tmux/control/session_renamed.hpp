#pragma once

#include <tmux_cxx/session/session_event.hpp>

#include <string>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Format one committed session rename as a stock control notification. */
std::string format_session_renamed(const SessionRenamedEvent &event);
} // namespace tmux_cxx::protocol::tmux::control
