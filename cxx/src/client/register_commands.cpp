#include "register_commands.hpp"

#include "operations/attach_session_command.hpp"

namespace tmux_cxx {
Result<void> register_client_commands(commands::CommandRegistry &registry) {
    return registry.add<AttachSessionCommand>();
}
} // namespace tmux_cxx
