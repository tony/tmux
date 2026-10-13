#include "commands/set_option_command.hpp"

#include "commands/session_target.hpp"
#include "session/session_options.hpp"
#include "window/window_options.hpp"

#include <algorithm>
#include <concepts>

namespace tmux_cxx {
namespace {
/** @brief Classify supported assignment flags independently of option names. */
enum class SetOptionFlag { global, window, target };
/** @brief Associate a short flag spelling with its assignment behavior. */
struct SetOptionFlagDescription {
    char spelling;      ///< Short flag letter within a group.
    SetOptionFlag flag; ///< Assignment behavior selected by that letter.
};
constexpr std::array set_option_flags{
    SetOptionFlagDescription{'g', SetOptionFlag::global},
    SetOptionFlagDescription{'w', SetOptionFlag::window},
    SetOptionFlagDescription{'t', SetOptionFlag::target}}; ///< Immutable supported flag catalog.

/** @brief Parse explicit assignment flags, then select the option's catalog-owned scope. */
Result<SetOptionCommand::Request> read_option_assignment(std::span<const std::string> arguments,
                                                         bool force_window_scope) {
    bool global = false;
    bool explicit_window = false;
    std::optional<std::string> target;
    std::size_t next = 0;
    while (next < arguments.size() && arguments[next].starts_with('-')) {
        const auto &group = arguments[next++];
        if (group == "--") {
            break;
        }
        if (group.size() == 1) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "empty set-option flag group"});
        }
        for (std::size_t index = 1; index < group.size(); ++index) {
            const auto flag = std::ranges::find(set_option_flags, group[index],
                                                &SetOptionFlagDescription::spelling);
            if (flag == set_option_flags.end()) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::invalid_argument, "unsupported set-option flag"});
            }
            switch (flag->flag) {
            case SetOptionFlag::global:
                if (global) {
                    return std::unexpected(
                        TmuxError{TmuxErrorCode::invalid_argument, "duplicate global option flag"});
                }
                global = true;
                break;
            case SetOptionFlag::window:
                if (explicit_window) {
                    return std::unexpected(
                        TmuxError{TmuxErrorCode::invalid_argument, "duplicate window option flag"});
                }
                explicit_window = true;
                break;
            case SetOptionFlag::target:
                if (target || index + 1 != group.size() || next == arguments.size()) {
                    return std::unexpected(TmuxError{
                        TmuxErrorCode::invalid_argument,
                        "set-option -t requires one following target and must end its flag group"});
                }
                target = arguments[next++];
                break;
            }
        }
    }
    if (global == target.has_value() || arguments.size() - next != 2) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument,
                      "set-option requires -g or one -t target, an option name, and a value"});
    }
    const auto &name = arguments[next];
    const auto &value = arguments[next + 1];
    if (!force_window_scope && !explicit_window && session_options::catalog().find(name)) {
        SetOptionCommand::SessionTarget owner = GlobalSessionOptions{};
        if (target) {
            owner = SetOptionCommand::SessionTargetText{std::move(*target)};
        }
        return SetOptionCommand::SessionAssignment{std::move(owner), name, value};
    }
    if (window_options::catalog().find(name) == nullptr) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "unsupported option for the selected scope"});
    }
    SetOptionCommand::WindowTarget owner = GlobalWindowOptions{};
    if (target) {
        auto membership = parse_session_window_target(*target);
        if (!membership) {
            return std::unexpected(membership.error());
        }
        if (!membership->index) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "window option target requires session:index"});
        }
        owner = std::move(*membership);
    }
    return SetOptionCommand::WindowAssignment{std::move(owner), name, value};
}

/** @brief Resolve a session assignment at execution and invoke the session-owned operation. */
Result<commands::CommandOutcome>
execute_session_assignment(commands::CommandInvocation &invocation,
                           SetOptionCommand::SessionAssignment request) {
    SessionOptionTarget target = GlobalSessionOptions{};
    if (const auto *session_target =
            std::get_if<SetOptionCommand::SessionTargetText>(&request.target)) {
        auto session = commands::resolve_session_target(invocation.server, session_target->session);
        if (!session) {
            return std::unexpected(session.error());
        }
        target = *session;
    }
    auto assignment = SetSessionOption::execute(
        invocation.server, {target, std::move(request.name), std::move(request.value)});
    if (!assignment) {
        return std::unexpected(assignment.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace

Result<SetOptionCommand::Request> SetOptionCommand::read(std::span<const std::string> arguments) {
    return read_option_assignment(arguments, false);
}

Result<SetOptionCommand::WindowAssignment>
SetOptionCommand::read_window(std::span<const std::string> arguments) {
    auto request = read_option_assignment(arguments, true);
    if (!request) {
        return std::unexpected(request.error());
    }
    return std::get<WindowAssignment>(std::move(*request));
}

Result<commands::CommandOutcome>
SetOptionCommand::execute_window(commands::CommandInvocation &invocation,
                                 WindowAssignment request) {
    WindowOptionTarget target = GlobalWindowOptions{};
    if (const auto *membership = std::get_if<SessionWindowTarget>(&request.target)) {
        auto link = resolve_window_link_target(invocation.server, *membership);
        if (!link) {
            return std::unexpected(link.error());
        }
        target = link->window;
    }
    auto assignment = SetWindowOption::execute(
        invocation.server, {target, std::move(request.name), std::move(request.value)});
    if (!assignment) {
        return std::unexpected(assignment.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}

Result<commands::CommandOutcome> SetOptionCommand::execute(commands::CommandInvocation &invocation,
                                                           Request request) {
    return std::visit(
        /** @brief Invoke the assignment's concrete operation after scope selection. */
        [&invocation](auto assignment) -> Result<commands::CommandOutcome> {
            if constexpr (std::same_as<decltype(assignment), SessionAssignment>) {
                return execute_session_assignment(invocation, std::move(assignment));
            } else {
                return execute_window(invocation, std::move(assignment));
            }
        },
        std::move(request));
}
} // namespace tmux_cxx
