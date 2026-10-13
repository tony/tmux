#include "window/register_commands.hpp"
#include "window/operations/new_window_command.hpp"
#include "window/operations/rename_window_command.hpp"
#include "window/operations/select_layout_command.hpp"

namespace tmux_cxx {
Result<void> register_window_commands(commands::CommandRegistry &registry) {
    if (auto registration = registry.add<NewWindowCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<RenameWindowCommand>(); !registration) {
        return registration;
    }
    return registry.add<SelectLayoutCommand>();
}
} // namespace tmux_cxx
