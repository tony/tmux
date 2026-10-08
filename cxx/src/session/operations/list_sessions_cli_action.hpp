#pragma once

#include "cli/action_registry.hpp"

namespace tmux_cxx {
/** @brief Adapt the `list` command-line spelling to the session list operation. */
struct ListSessionsCliAction {
    /** @brief Represent the argument-free session listing request. */
    struct Request {};
    static constexpr cli::ActionDescription description{"list"}; ///< CLI syntax.
    /** @brief Reject arguments because session listing has no command-line options. */
    static Result<Request> read(std::span<const std::string_view> arguments);
    /** @brief List sessions through the native controller and print their identities and names. */
    static Result<void> execute(cli::ActionInvocation &invocation, Request request);
};
} // namespace tmux_cxx
