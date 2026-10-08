#include "session_target.hpp"

namespace tmux_cxx::commands {
Result<SessionId> resolve_session_target(const Server &server, std::string_view target) {
    for (const auto &session : server.sessions()) {
        if (session.name == target || "$" + std::to_string(session.id.value) == target) {
            return session.id;
        }
    }
    return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
}
} // namespace tmux_cxx::commands
