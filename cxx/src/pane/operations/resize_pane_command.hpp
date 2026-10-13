#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/pane/operations/resize_pane.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Adapt one declared stock border movement to the shared ResizePane operation. */
struct ResizePaneCommand {
    using Request = ResizePane::Request; ///< @copybrief ResizePane::Request
    static constexpr std::array<std::string_view, 1> aliases{"resizep"}; ///< Stock alias.
    static constexpr std::array operations{ResizePane::description.id};  ///< Required operation.
    static constexpr commands::CommandDescription description{
        "resize-pane", aliases, operations}; ///< Stock spellings and required operation.
    /** @brief Require one direction, an exact pane target, and an optional positive count. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Invoke the shared pane operation and return a completed command. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
