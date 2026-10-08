#include "connection/stock_server_operation_catalog.hpp"
#include "connection/stock_server_operation_registry.hpp"

#include <tmux_cxx/session/operations/list_stock_server_session_names.hpp>

#include <gtest/gtest.h>

namespace tmux_cxx::detail {
/** @brief Entity registration yields an immutable reverse catalog and closes its registry. */
TEST(StockServerOperationCatalog, SessionAdapterOwnsRegistration) {
    auto catalog = build_stock_server_operation_catalog();
    ASSERT_TRUE(catalog);
    EXPECT_EQ(catalog->commands(protocol::tmux::ConnectionMode::command),
              (std::vector<std::string>{"list-sessions"}));
    EXPECT_TRUE(catalog->commands(protocol::tmux::ConnectionMode::interactive).empty());
    StockServerOperationRegistry registry;
    ASSERT_TRUE(registry.add<ListStockServerSessionNames>());
    auto isolated_catalog = std::move(registry).freeze();
    ASSERT_TRUE(isolated_catalog);
    EXPECT_FALSE(registry.add<ListStockServerSessionNames>());
}
} // namespace tmux_cxx::detail
