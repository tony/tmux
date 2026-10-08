#include "client/register_commands.hpp"
#include "client/register_operations.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "commands/command_registry.hpp"
#include "pane/register_commands.hpp"
#include "pane/register_operations.hpp"
#include "paste_buffer/register_commands.hpp"
#include "paste_buffer/register_operations.hpp"
#include "protocol/native/operation_registry.hpp"
#include "session/operations/create_session_command.hpp"
#include "session/register_commands.hpp"
#include "session/register_operations.hpp"
#include <tmux_cxx/server/server.hpp>
#include <tmux_cxx/session/operations/list_sessions.hpp>

#include <gtest/gtest.h>

namespace tmux_cxx {
/** @brief Stock aliases invoke registered operations and reject unsupported syntax before effects.
 */
TEST(CommandRegistry, AliasesAndValidationShareServerOperations) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(register_session_operations(operations));
    ASSERT_TRUE(register_client_operations(operations));
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_session_commands(command_registry));
    ASSERT_TRUE(register_client_commands(command_registry));
    auto command_catalog = std::move(command_registry).freeze(*operation_catalog);
    ASSERT_TRUE(command_catalog);
    Server server;
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};
    std::vector<std::string> create{"new-session", "-d", "-s", "registered", "exec cat"};
    ASSERT_TRUE(command_catalog->invoke(invocation, create));
    std::vector<std::string> rename{"rename", "-t", "registered", "next"};
    ASSERT_TRUE(command_catalog->invoke(invocation, rename));
    std::vector<std::string> list{"ls", "-F", "#{session_name}:#{pane_id}"};
    auto output = command_catalog->invoke(invocation, list);
    ASSERT_TRUE(output);
    EXPECT_EQ(output->output, "next:%1\n");
    EXPECT_EQ(output->continuation, commands::ClientContinuation::exit);
    std::vector<std::string> invalid{"new-session", "-d", "-s", "another", "cat", "ignored"};
    EXPECT_FALSE(command_catalog->invoke(invocation, invalid));
    EXPECT_EQ(server.sessions().size(), 1);
    EXPECT_FALSE(command_registry.add<CreateSessionCommand>());
}

/** @brief Startup rejects a stock command whose native operation was not registered. */
TEST(CommandRegistry, MissingNativeOperationPreventsStartup) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<ListSessions>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_session_commands(command_registry));
    EXPECT_FALSE(std::move(command_registry).freeze(*operation_catalog));
}

/** @brief Registered split-window syntax reaches SplitPane without command-specific server logic.
 */
TEST(CommandRegistry, SplitWindowUsesTypedPaneOperation) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(register_session_operations(operations));
    ASSERT_TRUE(register_pane_operations(operations));
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_session_commands(command_registry));
    ASSERT_TRUE(register_pane_commands(command_registry));
    auto command_catalog = std::move(command_registry).freeze(*operation_catalog);
    ASSERT_TRUE(command_catalog);
    Server server;
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};
    std::vector<std::string> create{"new-session", "-d", "-s", "split", "/bin/cat"};
    ASSERT_TRUE(command_catalog->invoke(invocation, create));
    std::vector<std::string> split{"splitw", "-h", "-t", "%1", "/bin/cat"};
    ASSERT_TRUE(command_catalog->invoke(invocation, split));
    const auto panes = server.panes(server.sessions().front().window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2);
    EXPECT_EQ(panes->at(0).columns, 40U);
    EXPECT_EQ(panes->at(1).columns, 39U);
    std::vector<std::string> select{"selectp", "-t", "%1"};
    ASSERT_TRUE(command_catalog->invoke(invocation, select));
    const auto selected = server.panes(server.sessions().front().window);
    ASSERT_TRUE(selected);
    EXPECT_TRUE(selected->at(0).active);
    EXPECT_FALSE(selected->at(1).active);
    std::vector<std::string> unsupported{"split-window", "-Z", "-t", "%1"};
    EXPECT_FALSE(command_catalog->invoke(invocation, unsupported));
    std::vector<std::string> invalid_select{"select-pane", "-L", "-t", "%2"};
    EXPECT_FALSE(command_catalog->invoke(invocation, invalid_select));
    EXPECT_EQ(server.panes(server.sessions().front().window)->size(), 2);
    EXPECT_EQ(server.window(server.sessions().front().window)->pane, PaneId{1});
}

/** @brief Named buffer commands share typed storage and reject unsupported flags before effects. */
TEST(CommandRegistry, PasteBufferCommandsUseTypedOperations) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(register_paste_buffer_operations(operations));
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_paste_buffer_commands(command_registry));
    auto command_catalog = std::move(command_registry).freeze(*operation_catalog);
    ASSERT_TRUE(command_catalog);
    Server server;
    const auto session = server.create_session("paste-command", "/bin/cat");
    ASSERT_TRUE(session);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};

    std::vector<std::string> set{"setb", "-b", "shared", "first\nsecond"};
    ASSERT_TRUE(command_catalog->invoke(invocation, set));
    std::vector<std::string> paste{"pasteb", "-t", "%1", "-b", "shared"};
    ASSERT_TRUE(command_catalog->invoke(invocation, paste));
    EXPECT_TRUE(server.read_pane_text(session->pane)->input_pending);
    EXPECT_EQ(server.read_paste_buffer("shared"), "first\nsecond");

    std::vector<std::string> unsupported{"paste-buffer", "-d", "-b", "shared", "-t", "%1"};
    EXPECT_FALSE(command_catalog->invoke(invocation, unsupported));
    EXPECT_EQ(server.read_paste_buffer("shared"), "first\nsecond");
    std::vector<std::string> remove{"deleteb", "-b", "shared"};
    ASSERT_TRUE(command_catalog->invoke(invocation, remove));
    EXPECT_FALSE(server.read_paste_buffer("shared"));
}
} // namespace tmux_cxx
