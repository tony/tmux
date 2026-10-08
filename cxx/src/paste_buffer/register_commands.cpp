#include "register_commands.hpp"

#include "operations/delete_buffer_command.hpp"
#include "operations/paste_buffer_command.hpp"
#include "operations/set_buffer_command.hpp"

namespace tmux_cxx {
Result<void> register_paste_buffer_commands(commands::CommandRegistry &registry) {
    if (auto registration = registry.add<SetBufferCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<DeleteBufferCommand>(); !registration) {
        return registration;
    }
    return registry.add<PasteBufferCommand>();
}
} // namespace tmux_cxx
