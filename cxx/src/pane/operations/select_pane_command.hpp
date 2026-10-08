#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/pane/operations/select_pane.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate declared stock select-pane syntax into the shared SelectPane operation. */
struct SelectPaneCommand {
    using Request = SelectPane::Request; ///< @copybrief SelectPane::Request
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"selectp"};
    /** @brief Borrowed spellings and the required pane operation identity. */
    static constexpr commands::CommandDescription description{"select-pane", aliases,
                                                              SelectPane::description.id};
    /** @brief Require exactly one explicit percent-prefixed pane target. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Execute shared pane selection and return no command-stream output. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
