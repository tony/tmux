#include "register_cli_actions.hpp"

#include "operations/create_session_cli_action.hpp"
#include "operations/list_sessions_cli_action.hpp"

namespace tmux_cxx {
Result<void> register_session_cli_actions(cli::ActionRegistry &registry) {
    if (auto registration = registry.add<CreateSessionCliAction>(); !registration) {
        return registration;
    }
    return registry.add<ListSessionsCliAction>();
}
} // namespace tmux_cxx
