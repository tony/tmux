#pragma once

#include <tmux_cxx/tmux_error.hpp>

#include <string>
#include <string_view>
#include <vector>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Parse one bounded control-mode command with quotes and backslash escapes. */
Result<std::vector<std::string>> parse_command_line(std::string_view line);
} // namespace tmux_cxx::protocol::tmux::control
