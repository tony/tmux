#pragma once

#include "commands/command_registry.hpp"
#include "window_link/window_target.hpp"
#include <tmux_cxx/window/operations/create_window.hpp>
#include <tmux_cxx/window_link/operations/select_window.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Compose window creation and optional selection for the declared new-window syntax. */
struct NewWindowCommand {
    /** @brief Retain creation arguments while resolving the session and index only at execution. */
    struct Request {
        SessionWindowTarget target; ///< Destination session and optional explicit index.
        std::string name;           ///< Explicit literal window name.
        std::string command;        ///< First pane's shell command.
        bool detached;              ///< Preserve current selection when the -d flag was supplied.
    };
    static constexpr std::array<std::string_view, 1> aliases{"neww"}; ///< Alternate tmux spelling.
    static constexpr std::array operations{
        CreateWindow::description.id,
        SelectWindow::description.id}; ///< Required native operations.
    /** @brief Borrow command spellings and every operation used by its execution. */
    static constexpr commands::CommandDescription description{"new-window", aliases, operations};
    /** @brief Require an explicit session and name, optional -d, and at most one shell command. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Create through typed operations, then select the new membership unless detached.
     *
     * A selection failure retains the created window and program. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
