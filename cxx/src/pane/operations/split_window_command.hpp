#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/pane/operations/split_pane.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate declared stock split-window syntax into the shared SplitPane operation. */
struct SplitWindowCommand {
    using Request = SplitPane::Request; ///< @copybrief SplitPane::Request
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"splitw"};
    /** @brief Borrowed spellings and the required pane operation identity. */
    static constexpr commands::CommandDescription description{"split-window", aliases,
                                                              SplitPane::description.id};
    /** @brief Require an exact pane target, optional orientation, and at most one command. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Execute the same layout and process mutation used by native clients. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
