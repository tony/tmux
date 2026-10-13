#include "resize_pane_command.hpp"

#include "pane/pane_target.hpp"

#include <charconv>
#include <limits>
#include <optional>

namespace tmux_cxx {
namespace {
/** @brief Return the resize grammar supported before any layout effects. */
TmuxError resize_pane_syntax_error() {
    return {TmuxErrorCode::invalid_argument,
            "supported syntax: resize-pane -L|-R|-U|-D -t %pane [cells]"};
}
} // namespace

Result<ResizePaneCommand::Request> ResizePaneCommand::read(std::span<const std::string> arguments) {
    std::optional<PaneId> pane;
    std::optional<PaneResizeDirection> direction;
    std::optional<std::uint32_t> cells;
    for (std::size_t index = 0; index < arguments.size(); ++index) {
        const auto &argument = arguments[index];
        if (argument == "-L" || argument == "-R" || argument == "-U" || argument == "-D") {
            if (direction) {
                return std::unexpected(resize_pane_syntax_error());
            }
            if (argument == "-L") {
                direction = PaneResizeDirection::left;
            } else if (argument == "-R") {
                direction = PaneResizeDirection::right;
            } else if (argument == "-U") {
                direction = PaneResizeDirection::up;
            } else {
                direction = PaneResizeDirection::down;
            }
        } else if (argument == "-t") {
            if (pane || ++index == arguments.size()) {
                return std::unexpected(resize_pane_syntax_error());
            }
            pane = parse_pane_target(arguments[index]);
            if (!pane) {
                return std::unexpected(resize_pane_syntax_error());
            }
        } else {
            std::uint32_t count = 0;
            const auto parsed =
                std::from_chars(argument.data(), argument.data() + argument.size(), count);
            if (cells || index + 1 != arguments.size() || parsed.ec != std::errc{} || count == 0 ||
                count > static_cast<std::uint32_t>(std::numeric_limits<std::int32_t>::max()) ||
                parsed.ptr != argument.data() + argument.size()) {
                return std::unexpected(resize_pane_syntax_error());
            }
            cells = count;
        }
    }
    if (!pane || !direction) {
        return std::unexpected(resize_pane_syntax_error());
    }
    return Request{*pane, *direction, cells.value_or(1)};
}

Result<commands::CommandOutcome> ResizePaneCommand::execute(commands::CommandInvocation &invocation,
                                                            Request request) {
    if (auto resized = ResizePane::execute(invocation.server, request); !resized) {
        return std::unexpected(resized.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
