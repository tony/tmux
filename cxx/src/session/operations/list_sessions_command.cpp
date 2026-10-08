#include "list_sessions_command.hpp"
#include "commands/session_format.hpp"

namespace tmux_cxx {
Result<ListSessionsCommand::Request>
ListSessionsCommand::read(std::span<const std::string> arguments) {
    if (arguments.empty()) {
        return Request{"#{session_name}"};
    }
    if (arguments.size() == 2 && arguments[0] == "-F") {
        return Request{arguments[1]};
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument, "unsupported list-sessions option"});
}
Result<commands::CommandOutcome>
ListSessionsCommand::execute(commands::CommandInvocation &invocation, Request request) {
    auto sessions = ListSessions::execute(invocation.server, {});
    if (!sessions) {
        return std::unexpected(sessions.error());
    }
    std::string command_output;
    for (const auto &session : *sessions) {
        auto line = commands::format_session(session, request.expression);
        if (!line) {
            return std::unexpected(line.error());
        }
        command_output += *line + "\n";
    }
    return commands::CommandOutcome{std::move(command_output), commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
