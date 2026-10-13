#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/window_link/operations/previous_window.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve an explicit stock session target before previous window selection. */
struct PreviousWindowCommand {
    /** @brief Retain the session target until execution against current state. */
    struct Request {
        std::string session; ///< Exact session name or dollar-prefixed identity.
    };
    static constexpr std::array<std::string_view, 1> aliases{"prev"}; ///< Stock tmux alias.
    static constexpr std::array operations{
        PreviousWindow::description.id}; ///< Shared navigation operation.
    /** @brief Borrow command spellings and the typed navigation dependency. */
    static constexpr commands::CommandDescription description{"previous-window", aliases,
                                                              operations};
    /** @brief Require one explicit session target and reject unsupported navigation flags. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the live session and invoke its shared navigation operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
