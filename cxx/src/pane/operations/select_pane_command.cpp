#include "select_pane_command.hpp"

#include "pane/pane_target.hpp"

namespace tmux_cxx {
namespace {
/** @brief Return the exact stock subset advertised by this command adapter. */
TmuxError select_pane_syntax_error() {
    return {TmuxErrorCode::invalid_argument, "supported syntax: select-pane -t %pane"};
}
} // namespace

Result<SelectPaneCommand::Request> SelectPaneCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-t") {
        return std::unexpected(select_pane_syntax_error());
    }
    const auto pane = parse_pane_target(arguments[1]);
    if (!pane) {
        return std::unexpected(select_pane_syntax_error());
    }
    return Request{*pane};
}

Result<commands::CommandOutcome> SelectPaneCommand::execute(commands::CommandInvocation &invocation,
                                                            Request request) {
    if (auto selected = SelectPane::execute(invocation.server, request); !selected) {
        return std::unexpected(selected.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
