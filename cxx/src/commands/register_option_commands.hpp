#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register commands whose immutable option definitions select more than one entity scope.
 */
Result<void> register_option_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
