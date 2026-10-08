#include "paste_buffer_command.hpp"

#include "pane/pane_target.hpp"

#include <optional>

namespace tmux_cxx {
namespace {
/** @brief Return the exact stock subset advertised by this command adapter. */
TmuxError paste_buffer_syntax_error() {
    return {TmuxErrorCode::invalid_argument, "supported syntax: paste-buffer -b name -t %pane"};
}
} // namespace

Result<PasteBufferCommand::Request>
PasteBufferCommand::read(std::span<const std::string> arguments) {
    std::optional<std::string> name;
    std::optional<PaneId> pane;
    for (std::size_t index = 0; index < arguments.size(); ++index) {
        const auto &option = arguments[index];
        if (option != "-b" && option != "-t") {
            return std::unexpected(paste_buffer_syntax_error());
        }
        if (++index == arguments.size()) {
            return std::unexpected(paste_buffer_syntax_error());
        }
        if (option == "-b") {
            if (name) {
                return std::unexpected(paste_buffer_syntax_error());
            }
            name = arguments[index];
        } else {
            if (pane) {
                return std::unexpected(paste_buffer_syntax_error());
            }
            pane = parse_pane_target(arguments[index]);
            if (!pane) {
                return std::unexpected(paste_buffer_syntax_error());
            }
        }
    }
    if (!name || !pane) {
        return std::unexpected(paste_buffer_syntax_error());
    }
    return Request{std::move(*name), *pane};
}

Result<commands::CommandOutcome>
PasteBufferCommand::execute(commands::CommandInvocation &invocation, Request request) {
    if (auto pasted = PasteBufferIntoPane::execute(invocation.server, std::move(request));
        !pasted) {
        return std::unexpected(pasted.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
