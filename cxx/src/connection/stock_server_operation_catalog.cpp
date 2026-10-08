#include "connection/stock_server_operation_catalog.hpp"

#include "connection/stock_server_operation_registry.hpp"
#include "session/register_stock_server_operations.hpp"

#include <utility>

namespace tmux_cxx::detail {
StockServerOperationCatalog::StockServerOperationCatalog(
    std::vector<protocol::tmux::StockServerOperationDescription> operations)
    : operations_(std::move(operations)) {}

std::vector<std::string>
StockServerOperationCatalog::commands(protocol::tmux::ConnectionMode mode) const {
    std::vector<std::string> commands;
    for (const auto &operation : operations_) {
        if (operation.mode == mode) {
            commands.emplace_back(operation.command);
        }
    }
    return commands;
}

Result<StockServerOperationCatalog> build_stock_server_operation_catalog() {
    StockServerOperationRegistry stock_server_operation_registry;
    if (auto registration =
            register_session_stock_server_operations(stock_server_operation_registry);
        !registration) {
        return std::unexpected(registration.error());
    }
    return std::move(stock_server_operation_registry).freeze();
}
} // namespace tmux_cxx::detail
