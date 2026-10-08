#include "list_sessions_cli_action.hpp"

#include <tmux_cxx/connection/server_connection.hpp>

#include <ostream>

namespace tmux_cxx {
Result<ListSessionsCliAction::Request>
ListSessionsCliAction::read(std::span<const std::string_view> arguments) {
    if (!arguments.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "list does not accept arguments"});
    }
    return Request{};
}

Result<void> ListSessionsCliAction::execute(cli::ActionInvocation &invocation, Request) {
    auto sessions = invocation.connection.sessions();
    if (!sessions) {
        return std::unexpected(sessions.error());
    }
    for (const auto &session : *sessions) {
        invocation.output << session.id.value << ' ' << session.name << '\n';
        if (!invocation.output) {
            return std::unexpected(TmuxError{TmuxErrorCode::io, "cannot write listed session"});
        }
    }
    return {};
}
} // namespace tmux_cxx
