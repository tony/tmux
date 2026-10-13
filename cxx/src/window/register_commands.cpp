#include "window/register_commands.hpp"
#include "window/operations/new_window_command.hpp"

namespace tmux_cxx {
Result<void> register_window_commands(commands::CommandRegistry &registry) {
    return registry.add<NewWindowCommand>();
}
} // namespace tmux_cxx
