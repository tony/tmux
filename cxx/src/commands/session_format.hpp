#pragma once

#include <tmux_cxx/session/session_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

namespace tmux_cxx::commands {
/** @brief Expand supported session format variables and reject undeclared expressions. */
Result<std::string> format_session(const SessionSnapshot &session, std::string_view expression);
} // namespace tmux_cxx::commands
