#include "window/operations/select_layout_command.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx {
Result<SelectLayoutCommand::Request>
SelectLayoutCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 3 || arguments[0] != "-t") {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument,
                      "supported syntax: select-layout -t session:index preset"});
    }
    auto target = parse_session_window_target(arguments[1]);
    if (!target) {
        return std::unexpected(target.error());
    }
    const auto preset = std::ranges::find(window_layout_presets, arguments[2],
                                          &WindowLayoutPresetDescription::name);
    if (preset == window_layout_presets.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "unsupported window layout preset"});
    }
    return Request{std::move(*target), preset->preset};
}

Result<commands::CommandOutcome>
SelectLayoutCommand::execute(commands::CommandInvocation &invocation, Request request) {
    const auto membership = resolve_window_link_target(invocation.server, request.target);
    if (!membership) {
        return std::unexpected(membership.error());
    }
    const auto arranged =
        SelectLayout::execute(invocation.server, {membership->window, request.preset});
    if (!arranged) {
        return std::unexpected(arranged.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
