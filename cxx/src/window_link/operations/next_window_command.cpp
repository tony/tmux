#include "window_link/operations/next_window_command.hpp"

#include "commands/session_target.hpp"

namespace tmux_cxx {
Result<NextWindowCommand::Request> NextWindowCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-t") {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "supported syntax: next-window -t session"});
    }
    return Request{arguments[1]};
}
Result<commands::CommandOutcome> NextWindowCommand::execute(commands::CommandInvocation &invocation,
                                                            Request request) {
    const auto session = commands::resolve_session_target(invocation.server, request.session);
    if (!session) {
        return std::unexpected(session.error());
    }
    const auto selected = NextWindow::execute(invocation.server, {*session});
    if (!selected) {
        return std::unexpected(selected.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
