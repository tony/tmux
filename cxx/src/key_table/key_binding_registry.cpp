#include "key_table/key_binding_registry.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::key_table {
Result<void> KeyBindingRegistry::add_registration(KeyBindingDescription description,
                                                  KeyTable::KeyBindingAdapter adapter) {
    if (registration_closed_) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "key binding registration is closed"});
    }
    if (description.table.empty() || description.command.empty() || !adapter) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "key binding description is incomplete"});
    }
    const auto duplicate = std::ranges::find_if(
        registrations_,
        /** @brief Match one existing registration by its tmux table and normalized key. */
        [&](const Registration &candidate) {
            return candidate.description.table == description.table &&
                   candidate.description.key == description.key;
        });
    if (duplicate != registrations_.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "duplicate key-table binding"});
    }
    registrations_.push_back({description, adapter});
    return {};
}

Result<KeyTableCatalog>
KeyBindingRegistry::freeze(const protocol::native::OperationCatalog &operations) && {
    registration_closed_ = true;
    if (registrations_.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "key binding registry is empty"});
    }
    for (const auto &registration : registrations_) {
        if (!operations.contains(registration.description.operation)) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "key binding requires an unregistered operation",
                                             std::string(registration.description.command)});
        }
    }
    std::ranges::sort(registrations_,
                      /** @brief Order bindings by tmux table and then normalized key. */
                      [](const Registration &left, const Registration &right) {
                          if (left.description.table != right.description.table) {
                              return left.description.table < right.description.table;
                          }
                          return left.description.key < right.description.key;
                      });
    std::vector<KeyTable> tables;
    for (std::size_t first = 0; first < registrations_.size();) {
        const auto name = registrations_[first].description.table;
        std::vector<KeyTable::Binding> bindings;
        std::size_t next = first;
        while (next < registrations_.size() && registrations_[next].description.table == name) {
            bindings.push_back({registrations_[next].description, registrations_[next].adapter});
            ++next;
        }
        tables.push_back(KeyTable(name, std::move(bindings)));
        first = next;
    }
    return KeyTableCatalog(std::move(tables));
}
} // namespace tmux_cxx::key_table
