#pragma once

#include "option/option_catalog.hpp"

#include <map>
#include <optional>
#include <string>

namespace tmux_cxx::options {
/** @brief Own mutable option values while borrowing immutable definitions from a catalog. */
class OptionTable {
  public:
    /** @brief Construct an empty table whose missing values may inherit from a parent. */
    OptionTable() = default;
    /** @brief Materialize every catalog default into a root option table. */
    static OptionTable defaults(const OptionCatalog &catalog);
    /** @brief Store a parsed value and report whether the effective table entry changed. */
    Result<bool> set(const ParsedOption &option);
    /** @brief Remove one local value so later lookup inherits or uses its default. */
    bool unset(const OptionDefinition &definition);
    /** @brief Resolve a value through this table, an optional parent, then its definition default.
     */
    Result<OptionValue> effective(const OptionDefinition &definition,
                                  const OptionTable *parent = nullptr) const;
    /** @brief Resolve one key option while preserving a configured None value. */
    Result<std::optional<key_table::KeyCode>> key(const OptionDefinition &definition,
                                                  const OptionTable *parent = nullptr) const;

  private:
    std::map<std::string, OptionValue, std::less<>>
        values_; ///< Local values indexed by stable canonical option name.
};
} // namespace tmux_cxx::options
