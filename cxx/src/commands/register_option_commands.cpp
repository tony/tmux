#include "commands/register_option_commands.hpp"
#include "commands/set_option_command.hpp"

namespace tmux_cxx {
Result<void> register_option_commands(commands::CommandRegistry &registry) {
    return registry.add<SetOptionCommand>();
}
} // namespace tmux_cxx
