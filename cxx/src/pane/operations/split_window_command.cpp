#include "split_window_command.hpp"

#include "pane/pane_target.hpp"

#include <optional>

namespace tmux_cxx {
namespace {
/** @brief Return the exact stock subset advertised by this command adapter. */
TmuxError split_window_syntax_error() {
    return {TmuxErrorCode::invalid_argument,
            "supported syntax: split-window [-h|-v] -t %pane [command]"};
}
} // namespace

Result<SplitWindowCommand::Request>
SplitWindowCommand::read(std::span<const std::string> arguments) {
    std::optional<PaneId> pane;
    std::optional<PaneSplitOrientation> explicit_orientation;
    std::optional<std::string> command;
    for (std::size_t index = 0; index < arguments.size(); ++index) {
        const auto &argument = arguments[index];
        if (argument == "-h" || argument == "-v") {
            if (explicit_orientation) {
                return std::unexpected(split_window_syntax_error());
            }
            explicit_orientation = argument == "-h" ? PaneSplitOrientation::left_right
                                                    : PaneSplitOrientation::top_bottom;
            continue;
        }
        if (argument == "-t") {
            if (pane || ++index == arguments.size()) {
                return std::unexpected(split_window_syntax_error());
            }
            pane = parse_pane_target(arguments[index]);
            if (!pane) {
                return std::unexpected(split_window_syntax_error());
            }
            continue;
        }
        if (argument.starts_with('-') || command || index + 1 != arguments.size()) {
            return std::unexpected(split_window_syntax_error());
        }
        command = argument;
    }
    if (!pane) {
        return std::unexpected(split_window_syntax_error());
    }
    return Request{*pane, explicit_orientation.value_or(PaneSplitOrientation::top_bottom),
                   command.value_or("/bin/sh")};
}

Result<commands::CommandOutcome>
SplitWindowCommand::execute(commands::CommandInvocation &invocation, Request request) {
    auto split = SplitPane::execute(invocation.server, std::move(request));
    if (!split) {
        return std::unexpected(split.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
