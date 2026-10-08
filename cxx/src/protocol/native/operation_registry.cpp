#include "operation_registry.hpp"
#include "handshake.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::protocol::native {
Result<void> OperationRegistry::add_registration(OperationDescription description,
                                                 OperationCatalog::OperationAdapter adapter) {
    if (registration_closed_ || description.id.value == 0 || description.name.empty() ||
        adapter == nullptr) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid or late native registration"});
    }
    for (const auto &registration : registrations_) {
        if (registration.description.id == description.id ||
            registration.description.name == description.name) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "duplicate native operation identity or name"});
        }
    }
    registrations_.push_back({description, adapter});
    return {};
}

Result<OperationCatalog>
OperationRegistry::freeze(std::span<const OperationDescription> required) && {
    if (registrations_.empty() || registrations_.size() > 1024) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid native operation catalog size"});
    }
    for (const auto &registration : registrations_) {
        if ((registration.description.required_capabilities & ~known_capabilities) != 0) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "undeclared native capability reference"});
        }
        if (registration.description.effect != OperationEffect::query &&
            registration.description.effect != OperationEffect::mutation &&
            registration.description.effect != OperationEffect::program_input) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "undeclared native operation effect"});
        }
    }
    for (std::size_t i = 0; i < required.size(); ++i) {
        const auto &description = required[i];
        if (std::ranges::find(required.first(i), description.id, &OperationDescription::id) !=
            required.first(i).end()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "duplicate public operation contract"});
        }
        const auto registered_operation = std::ranges::find(
            registrations_, description.id,
            /** @brief Resolve the public wire identity against immutable registration metadata. */
            [](const OperationCatalog::Entry &registration) {
                return registration.description.id;
            });
        if (registered_operation == registrations_.end() ||
            registered_operation->description.name != description.name ||
            registered_operation->description.effect != description.effect ||
            registered_operation->description.required_capabilities !=
                description.required_capabilities) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument,
                          "public operation is missing or its registration differs",
                          std::string(description.name)});
        }
    }
    std::ranges::sort(
        registrations_, {},
        /** @brief Order immutable registrations by wire identity for bounded lookup. */
        [](const OperationCatalog::Entry &registration) {
            return registration.description.id.value;
        });
    registration_closed_ = true;
    return OperationCatalog(std::move(registrations_));
}
} // namespace tmux_cxx::protocol::native
