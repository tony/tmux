#include "attach_session_command.hpp"

#include "commands/session_target.hpp"

namespace tmux_cxx {
Result<AttachSessionCommand::Request>
AttachSessionCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-t") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: attach-session -t target"});
    }
    return Request{arguments[1]};
}

Result<commands::CommandOutcome>
AttachSessionCommand::execute(commands::CommandInvocation &invocation, Request request) {
    auto session = commands::resolve_session_target(invocation.server, request.target);
    if (!session) {
        return std::unexpected(session.error());
    }
    auto attached = AttachClient::execute(invocation.server, {invocation.client, *session});
    if (!attached) {
        return std::unexpected(attached.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::attached};
}
} // namespace tmux_cxx
