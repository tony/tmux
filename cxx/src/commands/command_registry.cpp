#include "command_registry.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::commands {
Result<void> CommandRegistry::add_registration(CommandDescription description,
                                               CommandCatalog::CommandAdapter adapter) {
    if (registration_closed_ || description.name.empty() || description.operations.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid or late command registration"});
    }
    for (std::size_t index = 0; index < description.operations.size(); ++index) {
        const auto operation = description.operations[index];
        const auto earlier = description.operations.first(index);
        if (operation.value == 0 || std::ranges::find(earlier, operation) != earlier.end()) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "empty or duplicate required command operation"});
        }
    }
    std::vector<std::string_view> registered_names{description.name};
    registered_names.insert(registered_names.end(), description.aliases.begin(),
                            description.aliases.end());
    std::ranges::sort(registered_names);
    if (registered_names.front().empty() ||
        std::ranges::adjacent_find(registered_names) != registered_names.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "duplicate or empty command spelling"});
    }
    for (auto name : registered_names) {
        for (const auto &registration : registrations_) {
            if (registration.name == name) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::invalid_argument, "duplicate command spelling"});
            }
        }
    }
    for (auto name : registered_names) {
        registrations_.push_back({name, description.operations, adapter});
    }
    return {};
}

Result<CommandCatalog>
CommandRegistry::freeze(const protocol::native::OperationCatalog &operations) && {
    if (registrations_.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "command catalog is empty"});
    }
    for (const auto &registration : registrations_) {
        for (const auto operation : registration.operations) {
            if (!operations.contains(operation)) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::invalid_argument,
                              "command references an unavailable native operation"});
            }
        }
    }
    std::vector<CommandCatalog::Entry> commands;
    commands.reserve(registrations_.size());
    for (const auto &registration : registrations_) {
        commands.push_back({registration.name, registration.adapter});
    }
    std::ranges::sort(commands, {}, &CommandCatalog::Entry::name);
    registration_closed_ = true;
    return CommandCatalog(std::move(commands));
}
} // namespace tmux_cxx::commands
