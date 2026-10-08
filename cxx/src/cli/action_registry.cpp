#include "cli/action_registry.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::cli {
Result<void> ActionRegistry::add_registration(ActionDescription description,
                                              ActionCatalog::ActionAdapter adapter) {
    if (registration_closed_) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action registry is closed"});
    }
    if (description.name.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action spelling cannot be empty"});
    }
    if (adapter == nullptr) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action adapter cannot be null"});
    }
    if (std::ranges::contains(registrations_, description.name, &ActionCatalog::Entry::name)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "duplicate CLI action spelling"});
    }
    registrations_.push_back({description.name, description.operands, adapter});
    return {};
}

Result<ActionCatalog> ActionRegistry::freeze() && {
    if (registration_closed_) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action registry is closed"});
    }
    if (registrations_.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "CLI action registry is empty"});
    }
    std::ranges::sort(registrations_, {}, &ActionCatalog::Entry::name);
    registration_closed_ = true;
    return ActionCatalog(std::move(registrations_));
}
} // namespace tmux_cxx::cli
