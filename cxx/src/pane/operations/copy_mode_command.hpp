#pragma once

#include "commands/command_registry.hpp"

#include <tmux_cxx/pane/operations/enter_copy_mode.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate explicit stock copy-mode entry into the shared pane operation. */
struct CopyModeCommand {
    using Request =
        EnterCopyMode::Request; ///< Owned pane target without a separate key-style override.
    static constexpr std::array<std::string_view, 0> aliases{}; ///< No alternate stock spelling.
    static constexpr std::array operations{EnterCopyMode::description.id};
    ///< Required native entry operation.
    static constexpr commands::CommandDescription description{"copy-mode", aliases, operations};
    ///< Stock spelling and shared operation dependency.
    /** @brief Require exactly one explicit percent-prefixed pane target before entry effects. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Enter shared pane mode and return no command-stream output. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
