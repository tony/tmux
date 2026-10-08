#pragma once

#include "connection/stock_server_operation_registry.hpp"

namespace tmux_cxx {
/** @brief Register typed session adapters used by C++ clients of stock tmux servers. */
Result<void>
register_session_stock_server_operations(detail::StockServerOperationRegistry &registry);
} // namespace tmux_cxx
