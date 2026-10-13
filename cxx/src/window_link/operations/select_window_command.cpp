#include "select_window_command.hpp"

namespace tmux_cxx {
Result<SelectWindowCommand::Request>
SelectWindowCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-t") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: select-window -t session:index"});
    }
    auto target = parse_session_window_target(arguments[1]);
    if (target && !target->index) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window selection requires an explicit index"});
    }
    return target;
}
Result<commands::CommandOutcome>
SelectWindowCommand::execute(commands::CommandInvocation &invocation, Request request) {
    const auto link = resolve_window_link_target(invocation.server, request);
    if (!link) {
        return std::unexpected(link.error());
    }
    if (auto selected = SelectWindow::execute(invocation.server, {link->id}); !selected) {
        return std::unexpected(selected.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
