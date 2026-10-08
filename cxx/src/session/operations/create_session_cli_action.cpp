#include "create_session_cli_action.hpp"

#include <tmux_cxx/connection/server_connection.hpp>

#include <ostream>
#include <utility>

namespace tmux_cxx {
Result<CreateSessionCliAction::Request>
CreateSessionCliAction::read(std::span<const std::string_view> arguments) {
    if (arguments.empty() || arguments.size() > 2) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "create requires NAME and optional COMMAND"});
    }
    return Request{std::string(arguments[0]),
                   arguments.size() == 2 ? std::string(arguments[1]) : "/bin/sh"};
}

Result<void> CreateSessionCliAction::execute(cli::ActionInvocation &invocation, Request request) {
    auto session =
        invocation.connection.create_session(std::move(request.name), std::move(request.command));
    if (!session) {
        return std::unexpected(session.error());
    }
    invocation.output << session->id.value << ' ' << session->pane.value << '\n';
    if (!invocation.output) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::io, "cannot write created session identities"});
    }
    return {};
}
} // namespace tmux_cxx
