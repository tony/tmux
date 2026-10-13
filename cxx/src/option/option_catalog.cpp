#include "option/option_catalog.hpp"

#include "key_table/single_byte_key_parser.hpp"

#include <algorithm>

namespace tmux_cxx::options {
const OptionDefinition *OptionCatalog::find(std::string_view name) const {
    const auto definition = std::ranges::find_if(
        definitions_,
        /** @brief Match an option definition by its canonical tmux name. */
        [name](const OptionDefinition *candidate) { return candidate->name == name; });
    return definition == definitions_.end() ? nullptr : *definition;
}

Result<ParsedOption> OptionCatalog::parse(std::string_view name, std::string_view value) const {
    const auto *definition = find(name);
    if (definition == nullptr) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "unsupported option: " + std::string(name)});
    }
    switch (definition->type) {
    case OptionValueType::key: {
        auto key = key_table::parse_single_byte_key(value);
        if (!key) {
            return std::unexpected(key.error());
        }
        if (*key) {
            return ParsedOption{definition, **key};
        }
        return ParsedOption{definition, std::monostate{}};
    }
    case OptionValueType::mode_keys: {
        const auto choice =
            std::ranges::find(window_mode_keys, value, &WindowModeKeysDescription::name);
        if (choice == window_mode_keys.end()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "mode-keys requires emacs or vi"});
        }
        return ParsedOption{definition, choice->mode_keys};
    }
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument, "unsupported option value type"});
}
} // namespace tmux_cxx::options
