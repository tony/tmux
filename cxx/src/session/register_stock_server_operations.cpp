#include "session/register_stock_server_operations.hpp"

#include <tmux_cxx/session/operations/list_stock_server_session_names.hpp>

namespace tmux_cxx {
Result<void>
register_session_stock_server_operations(detail::StockServerOperationRegistry &registry) {
    return registry.add<ListStockServerSessionNames>();
}
} // namespace tmux_cxx
