#pragma once

#include "cli/action_registry.hpp"

#include <string>

namespace tmux_cxx {
/** @brief Adapt the `create` command-line spelling to the session create operation. */
struct CreateSessionCliAction {
    /** @brief Own a session name and command before contacting the server. */
    struct Request {
        std::string name;    ///< Session name supplied by the caller.
        std::string command; ///< Shell command, defaulting to `/bin/sh`.
    };
    static constexpr cli::ActionDescription description{"create",
                                                        "NAME [COMMAND]"}; ///< CLI syntax.
    /** @brief Decode a required session name and optional shell command. */
    static Result<Request> read(std::span<const std::string_view> arguments);
    /** @brief Create the session through the native controller and print its identities. */
    static Result<void> execute(cli::ActionInvocation &invocation, Request request);
};
} // namespace tmux_cxx
