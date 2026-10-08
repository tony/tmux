#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register stock commands that mutate a connection-lifetime client. */
Result<void> register_client_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
