#include "pane/operations/copy_mode_command.hpp"

#include "pane/pane_target.hpp"

namespace tmux_cxx {
Result<CopyModeCommand::Request> CopyModeCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() == 2 && arguments[0] == "-t") {
        if (const auto pane = parse_pane_target(arguments[1])) {
            return Request{*pane};
        }
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument, "supported syntax: copy-mode -t %pane"});
}

Result<commands::CommandOutcome> CopyModeCommand::execute(commands::CommandInvocation &invocation,
                                                          Request request) {
    if (auto entered = EnterCopyMode::execute(invocation.server, request); !entered) {
        return std::unexpected(entered.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
