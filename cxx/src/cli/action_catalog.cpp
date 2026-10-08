#include "action_catalog.hpp"

#include "cli/action_registry.hpp"
#include "session/register_cli_actions.hpp"

#include <algorithm>
#include <ostream>
#include <utility>

namespace tmux_cxx::cli {
ActionCatalog::ActionCatalog(std::vector<Entry> actions) : actions_(std::move(actions)) {}

Result<void> ActionCatalog::invoke(ActionInvocation &invocation,
                                   std::span<const std::string_view> arguments) const {
    if (arguments.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action spelling is required"});
    }
    const auto action = std::ranges::lower_bound(actions_, arguments.front(), {}, &Entry::name);
    if (action == actions_.end() || action->name != arguments.front()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "unsupported CLI action"});
    }
    auto action_result = action->adapter(invocation, arguments.subspan(1));
    if (!action_result && action_result.error().operation.empty()) {
        action_result.error().operation = action->name;
    }
    return action_result;
}

Result<void> ActionCatalog::write_usage(std::ostream &output, std::string_view prefix) const {
    for (const auto &action : actions_) {
        output << prefix << action.name;
        if (!action.operands.empty()) {
            output << ' ' << action.operands;
        }
        output << '\n';
    }
    if (!output) {
        return std::unexpected(TmuxError{TmuxErrorCode::io, "cannot write CLI usage"});
    }
    return {};
}

Result<ActionCatalog> build_action_catalog() {
    ActionRegistry action_registry;
    if (auto registration = register_session_cli_actions(action_registry); !registration) {
        return std::unexpected(registration.error());
    }
    return std::move(action_registry).freeze();
}
} // namespace tmux_cxx::cli
