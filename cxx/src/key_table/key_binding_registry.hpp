#pragma once

#include "key_table/key_table_catalog.hpp"
#include "protocol/native/operation_catalog.hpp"

#include <concepts>

namespace tmux_cxx::key_table {
/**
 * @brief Require immutable binding metadata and typed operation execution.
 * A backpressure result must precede effects because client input preserves and retries its key.
 */
template <class Binding>
concept RegisteredKeyBinding = requires(KeyBindingInvocation &invocation) {
    { Binding::description } -> std::convertible_to<KeyBindingDescription>;
    { Binding::execute(invocation) } -> std::same_as<Result<KeyBindingEffect>>;
};

/** @brief Validate typed key bindings before constructing immutable named tables. */
class KeyBindingRegistry {
  public:
    /** @brief Register one typed binding whose immutable description outlives the catalog. */
    template <RegisteredKeyBinding Binding> Result<void> add() {
        return add_registration(Binding::description, &Binding::execute);
    }
    /** @brief Consume registrations after every binding resolves to a native operation. */
    Result<KeyTableCatalog> freeze(const protocol::native::OperationCatalog &operations) &&;

  private:
    /** @brief Retain one mutable startup registration until validation completes. */
    struct Registration {
        KeyBindingDescription description;   ///< Borrowed immutable binding metadata.
        KeyTable::KeyBindingAdapter adapter; ///< Typed operation-backed entry point.
    };
    /** @brief Reject empty, duplicate, or late binding descriptions before storing them. */
    Result<void> add_registration(KeyBindingDescription description,
                                  KeyTable::KeyBindingAdapter adapter);
    std::vector<Registration> registrations_; ///< Mutable startup registrations.
    bool registration_closed_ = false;        ///< Prevent reuse after catalog construction.
};
} // namespace tmux_cxx::key_table
