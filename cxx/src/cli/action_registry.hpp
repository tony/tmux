#pragma once

#include "cli/action_catalog.hpp"

#include <concepts>
#include <utility>

namespace tmux_cxx::cli {

/** @brief Require owned argument decoding and typed execution for one CLI action. */
template <class Action>
concept RegisteredAction =
    requires(ActionInvocation &invocation, std::span<const std::string_view> arguments,
             typename Action::Request request) {
        { Action::description } -> std::convertible_to<ActionDescription>;
        { Action::read(arguments) } -> std::same_as<Result<typename Action::Request>>;
        { Action::execute(invocation, std::move(request)) } -> std::same_as<Result<void>>;
    };

/** @brief Validate and freeze typed CLI action registrations before process dispatch. */
class ActionRegistry {
  public:
    /** @brief Register one typed action whose immutable description outlives the resulting catalog.
     */
    template <RegisteredAction Action> Result<void> add() {
        /** @brief Decode this action's owned request before invoking its typed adapter. */
        auto action_adapter = +[](ActionInvocation &invocation,
                                  std::span<const std::string_view> arguments) -> Result<void> {
            auto request = Action::read(arguments);
            if (!request) {
                return std::unexpected(request.error());
            }
            return Action::execute(invocation, std::move(*request));
        };
        return add_registration(Action::description, action_adapter);
    }
    /** @brief Consume a nonempty validated registry into an immutable action catalog. */
    Result<ActionCatalog> freeze() &&;

  private:
    /** @brief Reject empty, duplicate, or late action registrations. */
    Result<void> add_registration(ActionDescription description,
                                  ActionCatalog::ActionAdapter adapter);
    std::vector<ActionCatalog::Entry> registrations_; ///< Mutable startup registrations.
    bool registration_closed_ = false;                ///< Prevent reuse after catalog construction.
};
} // namespace tmux_cxx::cli
