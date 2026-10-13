#pragma once

#include "commands/command_registry.hpp"
#include "window_link/window_target.hpp"
#include <tmux_cxx/window_link/operations/select_window.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate exact stock window targets into session-owned membership selection. */
struct SelectWindowCommand {
    using Request = SessionWindowTarget; ///< Target retained until execution-time resolution.
    static constexpr std::array<std::string_view, 1> aliases{
        "selectw"}; ///< Alternate tmux spelling.
    static constexpr std::array operations{
        SelectWindow::description.id}; ///< Required native operation.
    /** @brief Borrow command spellings and the required selection operation. */
    static constexpr commands::CommandDescription description{"select-window", aliases, operations};
    /** @brief Accept only an explicit -t session:index selection target. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the membership and select through the same operation used by native clients.
     */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
