#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/session/operations/create_session.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate declared stock new-session syntax into the shared CreateSession operation. */
struct CreateSessionCommand {
    using Request = CreateSession::Request; ///< @copybrief CreateSession::Request
    /** @brief Declared alternate stock spellings for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"new"};
    /** @brief Borrowed spellings and the required native operation identity. */
    static constexpr commands::CommandDescription description{"new-session", aliases,
                                                              CreateSession::description.id};
    /** @brief Accept detached named sessions and an optional shell command without ignoring extra
     * options. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Execute the same creation validation used by native clients. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
