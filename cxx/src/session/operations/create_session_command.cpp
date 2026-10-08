#include "create_session_command.hpp"

namespace tmux_cxx {
Result<CreateSessionCommand::Request>
CreateSessionCommand::read(std::span<const std::string> arguments) {
    if ((arguments.size() != 3 && arguments.size() != 4) || arguments[0] != "-d" ||
        arguments[1] != "-s") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: new-session -d -s name [command]"});
    }
    return Request{arguments[2], arguments.size() == 4 ? arguments[3] : "/bin/sh"};
}
Result<commands::CommandOutcome>
CreateSessionCommand::execute(commands::CommandInvocation &invocation, Request request) {
    auto created = CreateSession::execute(invocation.server, std::move(request));
    if (!created) {
        return std::unexpected(created.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
