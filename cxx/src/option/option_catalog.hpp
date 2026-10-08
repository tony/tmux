#pragma once

#include <tmux_cxx/key_table/key_code.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <array>
#include <cstddef>
#include <span>
#include <string_view>
#include <variant>

namespace tmux_cxx::options {
/** @brief Identify the tmux entity level that owns an option definition. */
enum class OptionScope { server, session, window, pane };

/** @brief Identify the value contract used to parse and store an option. */
enum class OptionValueType { key };

/** @brief Hold one value from the currently implemented tmux option types. */
using OptionValue = std::variant<std::monostate, key_table::KeyCode>;

/** @brief Describe one immutable tmux option and its default value. */
struct OptionDefinition {
    std::string_view name;     ///< Canonical tmux option name.
    OptionScope scope;         ///< Entity level owning runtime values.
    OptionValueType type;      ///< Parser and accessor contract.
    OptionValue default_value; ///< Value used when no table supplies an override.
};

/** @brief Report whether a value satisfies an option definition's declared type. */
constexpr bool option_value_matches(const OptionDefinition &definition, const OptionValue &value) {
    switch (definition.type) {
    case OptionValueType::key:
        return std::holds_alternative<std::monostate>(value) ||
               std::holds_alternative<key_table::KeyCode>(value);
    }
    return false;
}

/** @brief Pair a catalog-owned definition with its validated runtime value. */
struct ParsedOption {
    const OptionDefinition *definition; ///< Catalog definition that parsed the value.
    OptionValue value;                  ///< Validated value ready for an option table.
};

/** @brief Verify one scope's unique option definitions and type-compatible defaults. */
template <std::size_t Size>
constexpr bool
valid_scoped_option_definitions(const std::array<const OptionDefinition *, Size> &definitions,
                                OptionScope scope) {
    for (std::size_t index = 0; index < definitions.size(); ++index) {
        const auto *definition = definitions[index];
        if (definition == nullptr || definition->name.empty() || definition->scope != scope) {
            return false;
        }
        if (!option_value_matches(*definition, definition->default_value)) {
            return false;
        }
        for (std::size_t earlier = 0; earlier < index; ++earlier) {
            if (definitions[earlier]->name == definition->name) {
                return false;
            }
        }
    }
    return true;
}

/** @brief Parse values through immutable option definitions without owning runtime state. */
class OptionCatalog {
  public:
    /** @brief Borrow definitions whose static lifetime exceeds this catalog. */
    constexpr explicit OptionCatalog(std::span<const OptionDefinition *const> definitions)
        : definitions_(definitions) {}
    /** @brief Find one exact canonical option name. */
    const OptionDefinition *find(std::string_view name) const;
    /** @brief Parse a value according to the named option's declared type. */
    Result<ParsedOption> parse(std::string_view name, std::string_view value) const;
    /** @brief Borrow definitions in their declared stable order. */
    constexpr std::span<const OptionDefinition *const> definitions() const { return definitions_; }

  private:
    std::span<const OptionDefinition *const>
        definitions_; ///< Static option definitions retained by the owning domain.
};
} // namespace tmux_cxx::options
