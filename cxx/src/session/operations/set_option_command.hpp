#pragma once

#include "commands/command_registry.hpp"

#include <tmux_cxx/session/operations/set_session_option.hpp>

#include <array>
#include <string>
#include <variant>

namespace tmux_cxx {
/** @brief Parse the supported stock set-option subset into a typed session operation. */
struct SetOptionCommand {
    /** @brief Retain an exact session target until execution resolves current server state. */
    struct NamedSessionTarget {
        std::string value; ///< Exact session name or dollar-prefixed identity.
    };
    /** @brief Distinguish global defaults from a session target awaiting resolution. */
    using Target = std::variant<GlobalSessionOptions, NamedSessionTarget>;
    /** @brief Carry one syntactically valid global or targeted session-option assignment. */
    struct Request {
        Target target;     ///< Global defaults or an unresolved exact session target.
        std::string name;  ///< Canonical implemented session option name.
        std::string value; ///< Tmux text parsed by the option catalog.
    };
    /** @brief Declare tmux's short set-option spelling. */
    static constexpr std::array<std::string_view, 1> aliases{"set"};
    /** @brief Map set-option spellings to the shared session-option operation. */
    static constexpr commands::CommandDescription description{"set-option", aliases,
                                                              SetSessionOption::description.id};
    /** @brief Require exact global or targeted assignment syntax before option parsing or effects.
     */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Execute one parsed assignment through the shared typed session operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
