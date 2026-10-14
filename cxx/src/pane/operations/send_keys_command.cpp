#include "pane/operations/send_keys_command.hpp"

#include "pane/pane_target.hpp"

#include <tmux_cxx/pane/operations/read_copy_mode.hpp>

#include <charconv>
#include <optional>

namespace tmux_cxx {
const commands::CommandDescription SendKeysCommand::description{"send-keys", aliases,
                                                                copy_mode_command_operations()};

namespace {
/** @brief Report supported send-keys syntax before any pane or mode effects. */
TmuxError send_keys_syntax_error() {
    return {TmuxErrorCode::invalid_argument,
            "supported syntax: send-keys -X [-N positive-count] -t %pane action"};
}
} // namespace

Result<SendKeysCommand::Request> SendKeysCommand::read(std::span<const std::string> arguments) {
    bool mode_action = false;
    std::optional<PaneId> pane;
    std::optional<std::uint32_t> count;
    std::optional<std::string_view> action;
    for (std::size_t index = 0; index < arguments.size(); ++index) {
        const auto &argument = arguments[index];
        if (argument == "-X") {
            if (mode_action) {
                return std::unexpected(send_keys_syntax_error());
            }
            mode_action = true;
        } else if (argument == "-t") {
            if (pane || ++index == arguments.size()) {
                return std::unexpected(send_keys_syntax_error());
            }
            pane = parse_pane_target(arguments[index]);
            if (!pane) {
                return std::unexpected(send_keys_syntax_error());
            }
        } else if (argument == "-N") {
            if (count || ++index == arguments.size()) {
                return std::unexpected(send_keys_syntax_error());
            }
            const auto &text = arguments[index];
            std::uint32_t value = 0;
            const auto parsed = std::from_chars(text.data(), text.data() + text.size(), value);
            if (parsed.ec != std::errc{} || parsed.ptr != text.data() + text.size() || value == 0) {
                return std::unexpected(send_keys_syntax_error());
            }
            count = value;
        } else {
            if (index + 1 != arguments.size() || argument.empty() || argument.front() == '-') {
                return std::unexpected(send_keys_syntax_error());
            }
            action = argument;
        }
    }
    if (!mode_action || !pane || !action) {
        return std::unexpected(send_keys_syntax_error());
    }
    // Stock repeat prefixes affect motions; boundary and selection factories accept only the pane.
    return read_copy_mode_command(*action, *pane, count.value_or(1));
}

Result<commands::CommandOutcome> SendKeysCommand::execute(commands::CommandInvocation &invocation,
                                                          Request request) {
    const auto observed =
        ReadCopyMode::execute(invocation.server, {copy_mode_command_pane(request)});
    if (!observed) {
        return std::unexpected(observed.error());
    }
    if (!observed->mode) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument, "not in a mode"});
    }
    if (auto executed = execute_copy_mode_command(invocation.server, request); !executed) {
        return std::unexpected(executed.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
