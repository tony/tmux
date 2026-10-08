#include "rename_session_command.hpp"
#include "commands/session_target.hpp"

namespace tmux_cxx {
Result<RenameSessionCommand::Request>
RenameSessionCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 3 || arguments[0] != "-t") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: rename-session -t target name"});
    }
    return Request{arguments[1], arguments[2]};
}
Result<commands::CommandOutcome>
RenameSessionCommand::execute(commands::CommandInvocation &invocation, Request request) {
    auto id = commands::resolve_session_target(invocation.server, request.target);
    if (!id) {
        return std::unexpected(id.error());
    }
    auto renamed = RenameSession::execute(invocation.server, {*id, std::move(request.name)});
    if (!renamed) {
        return std::unexpected(renamed.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
