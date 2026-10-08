#pragma once

#include "connection/stock_server_operation_catalog.hpp"

namespace tmux_cxx::detail {
/** @brief Validate typed reverse adapters before constructing their capability catalog. */
class StockServerOperationRegistry {
  public:
    /** @brief Register one entity-owned reverse adapter during startup composition. */
    template <protocol::tmux::RegisteredStockServerOperation Operation> Result<void> add() {
        return add_description(Operation::description);
    }
    /** @brief Consume a nonempty validated registry into an immutable reverse catalog. */
    Result<StockServerOperationCatalog> freeze() &&;

  private:
    /** @brief Reject empty, duplicate, unsupported, or late reverse-adapter descriptions. */
    Result<void> add_description(protocol::tmux::StockServerOperationDescription description);
    std::vector<protocol::tmux::StockServerOperationDescription>
        registrations_;                ///< Mutable entity-owned reverse-operation registrations.
    bool registration_closed_ = false; ///< Prevent reuse after catalog construction.
};
} // namespace tmux_cxx::detail
