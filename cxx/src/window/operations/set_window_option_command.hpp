#pragma once

#include "commands/set_option_command.hpp"

namespace tmux_cxx {
/** @brief Adapt set-window-option spellings through the shared scoped assignment parser. */
struct SetWindowOptionCommand {
    using Request = SetOptionCommand::WindowAssignment; ///< Window assignment awaiting resolution.
    static constexpr std::array<std::string_view, 1> aliases{"setw"}; ///< Stock tmux alias.
    static constexpr std::array operations{
        SetWindowOption::description.id}; ///< Required operation.
    /** @brief Declare window-option spellings and their shared native dependency. */
    static constexpr commands::CommandDescription description{"set-window-option", aliases,
                                                              operations};
    /** @brief Require global defaults or an indexed membership and a window option. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the shared window and invoke its registered assignment operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
