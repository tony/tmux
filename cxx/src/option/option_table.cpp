#include "option/option_table.hpp"

namespace tmux_cxx::options {
OptionTable OptionTable::defaults(const OptionCatalog &catalog) {
    OptionTable table;
    for (const auto *definition : catalog.definitions()) {
        table.values_.emplace(definition->name, definition->default_value);
    }
    return table;
}

Result<bool> OptionTable::set(const ParsedOption &option) {
    if (option.definition == nullptr || !option_value_matches(*option.definition, option.value)) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "option value does not match its definition"});
    }
    const auto existing = values_.find(option.definition->name);
    if (existing != values_.end() && existing->second == option.value) {
        return false;
    }
    values_.insert_or_assign(std::string(option.definition->name), option.value);
    return true;
}

bool OptionTable::unset(const OptionDefinition &definition) {
    const auto local = values_.find(definition.name);
    if (local == values_.end()) {
        return false;
    }
    values_.erase(local);
    return true;
}

Result<OptionValue> OptionTable::effective(const OptionDefinition &definition,
                                           const OptionTable *parent) const {
    const auto local = values_.find(definition.name);
    if (local != values_.end()) {
        if (!option_value_matches(definition, local->second)) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "stored option has an incompatible value"});
        }
        return local->second;
    }
    if (parent != nullptr) {
        return parent->effective(definition);
    }
    return definition.default_value;
}

Result<std::optional<key_table::KeyCode>> OptionTable::key(const OptionDefinition &definition,
                                                           const OptionTable *parent) const {
    if (definition.type != OptionValueType::key) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "option is not a key value"});
    }
    auto value = effective(definition, parent);
    if (!value) {
        return std::unexpected(value.error());
    }
    if (std::holds_alternative<std::monostate>(*value)) {
        return std::optional<key_table::KeyCode>{};
    }
    return std::get<key_table::KeyCode>(*value);
}
} // namespace tmux_cxx::options
