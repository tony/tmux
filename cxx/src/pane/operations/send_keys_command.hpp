#pragma once

#include "commands/command_registry.hpp"
#include "pane/operations/copy_mode_command_catalog.hpp"

namespace tmux_cxx {
/** @brief Translate declared send-keys mode actions into a closed native copy request. */
struct SendKeysCommand {
    using Request =
        CopyModeCommandRequest; ///< Concrete request selected after complete argument validation.
    static constexpr std::array<std::string_view, 1> aliases{
        "send"}; ///< Exact stock alternate spelling.
    static const commands::CommandDescription description;
    ///< Immutable stock spelling and dependencies derived from the copy-command catalog.
    /** @brief Require -X, explicit pane target, optional positive repeat count, and one supported
     * action. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Reject inactive pane modes before executing the selected native copy request. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
