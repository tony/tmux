#pragma once

#include <tmux_cxx/session/session_snapshot.hpp>

#include <string>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Encode one attached-session selection as a stock control notification. */
std::string format_session_changed(const SessionSnapshot &session);
} // namespace tmux_cxx::protocol::tmux::control
