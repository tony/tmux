#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/client/operations/attach_client.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve stock attach-session syntax into the shared client attachment operation. */
struct AttachSessionCommand {
    /** @brief Session target resolved against current server state during execution. */
    struct Request {
        std::string target; ///< Exact session name or dollar-prefixed identity.
    };
    /** @brief Declared alternate stock spellings for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"attach"};
    /** @brief Borrowed spellings and the required client attachment operation. */
    static constexpr commands::CommandDescription description{"attach-session", aliases,
                                                              AttachClient::description.id};
    /** @brief Require one explicit target and reject unsupported attach options. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Attach the invoking client and keep its stock connection alive. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
