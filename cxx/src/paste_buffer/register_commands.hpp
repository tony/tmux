#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Add the declared paste-buffer stock commands to a mutable catalog. */
Result<void> register_paste_buffer_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
