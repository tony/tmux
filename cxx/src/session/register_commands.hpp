#pragma once

#include "commands/command_registry.hpp"

namespace tmux_cxx {
/** @brief Register supported stock session commands before validating and freezing their catalog.
 */
Result<void> register_session_commands(commands::CommandRegistry &registry);
} // namespace tmux_cxx
