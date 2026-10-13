#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/window_link/operations/next_window.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve an explicit stock session target before next window selection. */
struct NextWindowCommand {
    /** @brief Retain the session target until execution against current state. */
    struct Request {
        std::string session; ///< Exact session name or dollar-prefixed identity.
    };
    static constexpr std::array<std::string_view, 1> aliases{"next"}; ///< Stock tmux alias.
    static constexpr std::array operations{
        NextWindow::description.id}; ///< Shared navigation operation.
    /** @brief Borrow command spellings and the typed navigation dependency. */
    static constexpr commands::CommandDescription description{"next-window", aliases, operations};
    /** @brief Require one explicit session target and reject unsupported navigation flags. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the live session and invoke its shared navigation operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
