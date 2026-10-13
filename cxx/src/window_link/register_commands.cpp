#include "window_link/register_commands.hpp"
#include "window_link/operations/next_window_command.hpp"
#include "window_link/operations/previous_window_command.hpp"
#include "window_link/operations/select_window_command.hpp"

namespace tmux_cxx {
Result<void> register_window_link_commands(commands::CommandRegistry &registry) {
    if (auto registration = registry.add<SelectWindowCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<NextWindowCommand>(); !registration) {
        return registration;
    }
    return registry.add<PreviousWindowCommand>();
}
} // namespace tmux_cxx
