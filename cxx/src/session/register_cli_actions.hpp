#pragma once

#include "cli/action_registry.hpp"

namespace tmux_cxx {
/** @brief Register session-owned command-line adapters before the CLI catalog freezes. */
Result<void> register_session_cli_actions(cli::ActionRegistry &registry);
} // namespace tmux_cxx
