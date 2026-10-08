#include "register_commands.hpp"
#include "operations/create_session_command.hpp"
#include "operations/list_sessions_command.hpp"
#include "operations/rename_session_command.hpp"
#include "operations/set_option_command.hpp"

namespace tmux_cxx {
Result<void> register_session_commands(commands::CommandRegistry &registry) {
    if (auto registration = registry.add<CreateSessionCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ListSessionsCommand>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<RenameSessionCommand>(); !registration) {
        return registration;
    }
    return registry.add<SetOptionCommand>();
}
} // namespace tmux_cxx
