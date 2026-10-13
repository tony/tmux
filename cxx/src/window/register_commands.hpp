#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register stock commands that create shared windows. */
Result<void> register_window_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
