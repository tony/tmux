#include "window_link/operations/previous_window_command.hpp"

#include "commands/session_target.hpp"

namespace tmux_cxx {
Result<PreviousWindowCommand::Request>
PreviousWindowCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-t") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: previous-window -t session"});
    }
    return Request{arguments[1]};
}
Result<commands::CommandOutcome>
PreviousWindowCommand::execute(commands::CommandInvocation &invocation, Request request) {
    const auto session = commands::resolve_session_target(invocation.server, request.session);
    if (!session) {
        return std::unexpected(session.error());
    }
    const auto selected = PreviousWindow::execute(invocation.server, {*session});
    if (!selected) {
        return std::unexpected(selected.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
