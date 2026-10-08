#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/session/operations/rename_session.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve stock rename-session targets before invoking the shared typed operation. */
struct RenameSessionCommand {
    /** @brief Target text resolved at execution time and a replacement session name. */
    struct Request {
        std::string target; ///< Exact session name or dollar-prefixed identity.
        std::string name;   ///< Replacement name validated before mutation.
    };
    /** @brief Declared alternate stock spellings for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"rename"};
    /** @brief Borrowed spellings and the required native operation identity. */
    static constexpr commands::CommandDescription description{"rename-session", aliases,
                                                              RenameSession::description.id};
    /** @brief Require an explicit target and one replacement name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the target against current state and reuse native rename validation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
