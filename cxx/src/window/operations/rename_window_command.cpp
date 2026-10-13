#include "window/operations/rename_window_command.hpp"

#include <utility>

namespace tmux_cxx {
Result<RenameWindowCommand::Request>
RenameWindowCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 3 || arguments[0] != "-t" ||
        arguments[2].find("#{") != arguments[2].npos) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: rename-window -t session:index name"});
    }
    auto target = parse_session_window_target(arguments[1]);
    if (!target) {
        return std::unexpected(target.error());
    }
    return Request{std::move(*target), arguments[2]};
}

Result<commands::CommandOutcome>
RenameWindowCommand::execute(commands::CommandInvocation &invocation, Request request) {
    const auto membership = resolve_window_link_target(invocation.server, request.target);
    if (!membership) {
        return std::unexpected(membership.error());
    }
    const auto renamed =
        RenameWindow::execute(invocation.server, {membership->window, std::move(request.name)});
    if (!renamed) {
        return std::unexpected(renamed.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
