#include "window_link/register_commands.hpp"
#include "window_link/operations/select_window_command.hpp"

namespace tmux_cxx {
Result<void> register_window_link_commands(commands::CommandRegistry &registry) {
    return registry.add<SelectWindowCommand>();
}
} // namespace tmux_cxx
