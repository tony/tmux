#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register stock commands selecting session-owned window memberships. */
Result<void> register_window_link_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
