#pragma once

#include "commands/command_registry.hpp"
#include "window_link/window_target.hpp"

#include <tmux_cxx/session/operations/set_session_option.hpp>
#include <tmux_cxx/window/operations/set_window_option.hpp>

#include <array>
#include <string>
#include <variant>

namespace tmux_cxx {
/** @brief Select set-option scope through immutable definitions before invoking typed assignments.
 */
struct SetOptionCommand {
    /** @brief Retain an exact session target until execution resolves current server state. */
    struct SessionTargetText {
        std::string session; ///< Exact session name or dollar-prefixed identity.
    };
    /** @brief Distinguish global session defaults from a session target awaiting resolution. */
    using SessionTarget = std::variant<GlobalSessionOptions, SessionTargetText>;
    /** @brief Distinguish global window defaults from an indexed membership awaiting resolution. */
    using WindowTarget = std::variant<GlobalWindowOptions, SessionWindowTarget>;
    /** @brief Retain one global or targeted session-option assignment until execution. */
    struct SessionAssignment {
        SessionTarget target; ///< Global defaults or an unresolved session target.
        std::string name;     ///< Canonical implemented session option name.
        std::string value;    ///< Tmux text parsed by the session option catalog.
    };
    /** @brief Retain one global or targeted window-option assignment until execution. */
    struct WindowAssignment {
        WindowTarget target; ///< Global defaults or an unresolved indexed membership.
        std::string name;    ///< Canonical implemented window option name.
        std::string value;   ///< Tmux text parsed by the window option catalog.
    };
    /** @brief Preserve the concrete option owner across parsing and target resolution. */
    using Request = std::variant<SessionAssignment, WindowAssignment>;
    static constexpr std::array<std::string_view, 1> aliases{"set"}; ///< Stock tmux alias.
    static constexpr std::array operations{
        SetSessionOption::description.id,
        SetWindowOption::description.id}; ///< Required operations.
    /** @brief Declare set-option spellings and every scoped operation its definitions may select.
     */
    static constexpr commands::CommandDescription description{"set-option", aliases, operations};
    /** @brief Require explicit global or targeted assignment and select its definition-owned scope.
     */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Parse the same flags while requiring a window-scoped option definition. */
    static Result<WindowAssignment> read_window(std::span<const std::string> arguments);
    /** @brief Resolve the assignment's current owner and invoke its typed operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
    /** @brief Resolve one indexed membership and assign its shared window option. */
    static Result<commands::CommandOutcome> execute_window(commands::CommandInvocation &invocation,
                                                           WindowAssignment request);
};
} // namespace tmux_cxx
