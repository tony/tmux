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
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument, "unsupported option"});
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
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument, "unsupported session option type"});
}
} // namespace tmux_cxx::options
