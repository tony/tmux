#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register stock commands that mutate panes through typed pane operations. */
Result<void> register_pane_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
