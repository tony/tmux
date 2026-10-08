#pragma once

#include "key_table/key_binding.hpp"

#include <optional>
#include <vector>

namespace tmux_cxx::key_table {
class KeyBindingRegistry;

/** @brief Own one immutable named table of validated operation-backed key bindings. */
class KeyTable {
  public:
    /** @brief Borrow this table's stable startup-registered name. */
    std::string_view name() const;

  private:
    friend class KeyBindingRegistry;
    friend class KeyTableCatalog;
    /** @brief Erase binding-specific execution only at immutable key lookup. */
    using KeyBindingAdapter = Result<KeyBindingEffect> (*)(KeyBindingInvocation &);
    /** @brief Retain validated binding metadata beside its typed adapter. */
    struct Binding {
        KeyBindingDescription description; ///< Borrowed table, command, and operation metadata.
        KeyBindingAdapter adapter;         ///< Typed binding invocation entry point.
    };
    /** @brief Adopt one sorted nonempty binding set for a startup-validated table. */
    KeyTable(std::string_view name, std::vector<Binding> bindings);
    std::string_view name_;         ///< Borrowed immutable table name.
    std::vector<Binding> bindings_; ///< Bindings sorted by normalized key value.
};

/** @brief Provide immutable lookup across startup-validated tmux key tables. */
class KeyTableCatalog {
  public:
    /** @brief Report whether one normalized key is bound in the named table. */
    bool contains(std::string_view table, KeyCode key) const;
    /** @brief Invoke one binding, returning an empty optional when the table or key is unbound. */
    Result<std::optional<KeyBindingEffect>> invoke(std::string_view table, KeyCode key,
                                                   KeyBindingInvocation &invocation) const;

  private:
    friend class KeyBindingRegistry;
    /** @brief Borrow one immutable binding by its table and normalized key. */
    const KeyTable::Binding *find_binding(std::string_view table, KeyCode key) const;
    /** @brief Adopt validated nonempty tables already sorted by name. */
    explicit KeyTableCatalog(std::vector<KeyTable> tables);
    std::vector<KeyTable> tables_; ///< Immutable named tables sorted for bounded lookup.
};
} // namespace tmux_cxx::key_table
