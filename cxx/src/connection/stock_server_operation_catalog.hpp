#pragma once

#include <tmux_cxx/protocol/tmux/stock_server_operation.hpp>

#include <string>
#include <vector>

namespace tmux_cxx::detail {
class StockServerOperationRegistry;

/** @brief Expose immutable reverse-operation capabilities after startup validation. */
class StockServerOperationCatalog {
  public:
    /** @brief Copy command spellings admitted for one reverse connection mode. */
    std::vector<std::string> commands(protocol::tmux::ConnectionMode mode) const;

  private:
    friend class StockServerOperationRegistry;
    /** @brief Adopt validated reverse-operation descriptions sorted by identity. */
    explicit StockServerOperationCatalog(
        std::vector<protocol::tmux::StockServerOperationDescription> operations);
    std::vector<protocol::tmux::StockServerOperationDescription>
        operations_; ///< Frozen entity-owned reverse-operation descriptions.
};

/** @brief Register every entity's reverse adapters and freeze their capability catalog. */
Result<StockServerOperationCatalog> build_stock_server_operation_catalog();
} // namespace tmux_cxx::detail
