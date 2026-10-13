#include "register_commands.hpp"

#include "operations/resize_pane_command.hpp"
#include "operations/select_pane_command.hpp"
#include "operations/split_window_command.hpp"

namespace tmux_cxx {
Result<void> register_pane_commands(commands::CommandRegistry &registry) {
    if (auto registration = registry.add<SplitWindowCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectPaneCommand>(); !registration) {
        return registration;
    }
    return registry.add<ResizePaneCommand>();
}
} // namespace tmux_cxx
