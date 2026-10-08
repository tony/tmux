#include "key_table/key_table_catalog.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::key_table {
KeyTable::KeyTable(std::string_view name, std::vector<Binding> bindings)
    : name_(name), bindings_(std::move(bindings)) {}

std::string_view KeyTable::name() const { return name_; }

KeyTableCatalog::KeyTableCatalog(std::vector<KeyTable> tables) : tables_(std::move(tables)) {}

const KeyTable::Binding *KeyTableCatalog::find_binding(std::string_view table_name,
                                                       KeyCode key) const {
    const auto table = std::ranges::lower_bound(tables_, table_name, {}, &KeyTable::name);
    if (table == tables_.end() || table->name() != table_name) {
        return nullptr;
    }
    const auto binding = std::ranges::lower_bound(
        table->bindings_, key.value, {},
        /** @brief Compare one binding through its normalized key value. */
        [](const KeyTable::Binding &candidate) { return candidate.description.key.value; });
    if (binding == table->bindings_.end() || binding->description.key != key) {
        return nullptr;
    }
    return &*binding;
}

bool KeyTableCatalog::contains(std::string_view table_name, KeyCode key) const {
    return find_binding(table_name, key) != nullptr;
}

Result<std::optional<KeyBindingEffect>>
KeyTableCatalog::invoke(std::string_view table_name, KeyCode key,
                        KeyBindingInvocation &invocation) const {
    const auto *binding = find_binding(table_name, key);
    if (!binding) {
        return std::optional<KeyBindingEffect>{};
    }
    auto effect = binding->adapter(invocation);
    if (!effect) {
        effect.error().operation = binding->description.command;
        return std::unexpected(effect.error());
    }
    return std::optional<KeyBindingEffect>{*effect};
}
} // namespace tmux_cxx::key_table
