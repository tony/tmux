#include "register_operations.hpp"

#include <tmux_cxx/session/operations/create_session.hpp>
#include <tmux_cxx/session/operations/kill_session.hpp>
#include <tmux_cxx/session/operations/list_sessions.hpp>
#include <tmux_cxx/session/operations/observe_sessions.hpp>
#include <tmux_cxx/session/operations/read_session_events.hpp>
#include <tmux_cxx/session/operations/rename_session.hpp>
#include <tmux_cxx/session/operations/set_session_option.hpp>
#include <tmux_cxx/session/operations/wait_session_events.hpp>

namespace tmux_cxx {
Result<void> register_session_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<CreateSession>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ListSessions>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ObserveSessions>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ReadSessionEvents>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<WaitSessionEvents>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<RenameSession>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<KillSession>(); !registration) {
        return registration;
    }
    return registry.add<SetSessionOption>();
}
} // namespace tmux_cxx
