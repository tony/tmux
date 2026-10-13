#pragma once

#include "commands/command_registry.hpp"
#include "window_link/window_target.hpp"
#include <tmux_cxx/window/operations/rename_window.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve a stock window target before renaming the shared window it retains. */
struct RenameWindowCommand {
    /** @brief Retain the textual membership target and literal replacement name until execution. */
    struct Request {
        SessionWindowTarget target; ///< Explicit session and its exact window index.
        std::string name;           ///< Literal window name validated by RenameWindow.
    };
    static constexpr std::array<std::string_view, 1> aliases{"renamew"}; ///< Stock tmux alias.
    static constexpr std::array operations{
        RenameWindow::description.id}; ///< Required shared-window operation.
    /** @brief Borrow stock spellings and the typed rename dependency for startup validation. */
    static constexpr commands::CommandDescription description{"rename-window", aliases, operations};
    /** @brief Require an explicit membership target and one literal replacement name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the current membership and invoke RenameWindow on its shared window. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
