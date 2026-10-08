#include "session/operations/set_option_command.hpp"

#include "commands/session_target.hpp"

namespace tmux_cxx {
Result<SetOptionCommand::Request> SetOptionCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() == 3 && arguments[0] == "-g") {
        return Request{GlobalSessionOptions{}, arguments[1], arguments[2]};
    }
    if (arguments.size() == 4 && arguments[0] == "-t") {
        return Request{NamedSessionTarget{arguments[1]}, arguments[2], arguments[3]};
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument,
                  "supported syntax: set-option -g name value or set-option -t target name value"});
}

Result<commands::CommandOutcome> SetOptionCommand::execute(commands::CommandInvocation &invocation,
                                                           Request request) {
    SessionOptionTarget target = GlobalSessionOptions{};
    if (const auto *named_session = std::get_if<NamedSessionTarget>(&request.target)) {
        auto session = commands::resolve_session_target(invocation.server, named_session->value);
        if (!session) {
            return std::unexpected(session.error());
        }
        target = *session;
    }
    auto option_assignment = SetSessionOption::execute(
        invocation.server, {target, std::move(request.name), std::move(request.value)});
    if (!option_assignment) {
        return std::unexpected(option_assignment.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
