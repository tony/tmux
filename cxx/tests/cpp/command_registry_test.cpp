#include "client/register_commands.hpp"
#include "client/register_operations.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "commands/command_registry.hpp"
#include "pane/register_commands.hpp"
#include "pane/register_operations.hpp"
#include "paste_buffer/register_commands.hpp"
#include "paste_buffer/register_operations.hpp"
#include "protocol/native/operation_registry.hpp"
#include "server/api_catalogs.hpp"
#include "session/operations/create_session_command.hpp"
#include "session/register_commands.hpp"
#include "session/register_operations.hpp"
#include "window/operations/new_window_command.hpp"
#include "window/operations/select_layout_command.hpp"
#include <tmux_cxx/server/server.hpp>
#include <tmux_cxx/session/operations/list_sessions.hpp>

#include <gtest/gtest.h>

namespace tmux_cxx {
/** @brief A composed command cannot start without every operation it may invoke. */
TEST(CommandRegistry, NewWindowRequiresCreationAndSelection) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<CreateWindow>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry commands;
    ASSERT_TRUE(commands.add<NewWindowCommand>());
    EXPECT_FALSE(std::move(commands).freeze(*operation_catalog));
}

/** @brief Named layout commands require their declared operation before catalog publication. */
TEST(CommandRegistry, SelectLayoutRequiresItsOperation) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<ListSessions>());
    const auto catalog = std::move(operations).freeze();
    ASSERT_TRUE(catalog);
    commands::CommandRegistry commands;
    ASSERT_TRUE(commands.add<SelectLayoutCommand>());
    EXPECT_FALSE(std::move(commands).freeze(*catalog));
}

/** @brief Window commands preserve detached creation and session-local selection of shared windows.
 */
TEST(CommandRegistry, WindowCommandsSelectSessionMemberships) {
    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    Server server;
    const auto first = server.create_session("first", "/bin/cat");
    const auto second = server.create_session("second", "/bin/cat");
    ASSERT_TRUE(first);
    ASSERT_TRUE(second);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};

    const std::vector<std::string> create{"new-window", "-d",     "-t",      "first:7",
                                          "-n",         "shared", "/bin/cat"};
    const auto created = catalogs->stock_commands.invoke(invocation, create);
    ASSERT_TRUE(created) << (created ? "" : created.error().message);
    const auto links = server.window_links(first->id);
    ASSERT_TRUE(links);
    ASSERT_EQ(links->size(), 2U);
    EXPECT_EQ(links->back().index, 7U);
    EXPECT_EQ(server.sessions().front().window, first->window);

    const std::vector<std::string> select{"selectw", "-t", "first:7"};
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, select));
    EXPECT_EQ(server.sessions().front().window, links->back().window);
    const auto shared = server.link_window(links->back().window, second->id, 4);
    ASSERT_TRUE(shared);
    EXPECT_EQ(server.sessions().back().window, second->window);

    const std::vector<std::string> select_shared{"select-window", "-t", "second:4"};
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, select_shared));
    const std::vector<std::string> select_original{"select-window", "-t", "first:0"};
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, select_original));
    const std::vector<std::string> rename{"renamew", "-t", "second:4", "tools:build.1"};
    const auto renamed = catalogs->stock_commands.invoke(invocation, rename);
    ASSERT_TRUE(renamed) << (renamed ? "" : renamed.error().message);
    EXPECT_EQ(server.window(links->back().window)->name, "tools:build.1");
    EXPECT_EQ(server.window(shared->window)->name, "tools:build.1");
    const std::vector<std::string> next{"next", "-t", "first"};
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, next));
    EXPECT_EQ(server.sessions().front().link, links->back().id);
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, next));
    EXPECT_EQ(server.sessions().front().link, first->link);
    const std::vector<std::string> previous{"prev", "-t", "first"};
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, previous));
    EXPECT_EQ(server.sessions().front().link, links->back().id);
    ASSERT_TRUE(catalogs->stock_commands.invoke(invocation, previous));
    EXPECT_EQ(server.sessions().front().window, first->window);
    EXPECT_EQ(server.sessions().back().window, shared->window);
}

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
    const std::vector<std::string> resize{"resizep", "-L", "-t", "%2", "5"};
    const auto resized = command_catalog->invoke(invocation, resize);
    ASSERT_TRUE(resized) << (resized ? "" : resized.error().message);
    EXPECT_EQ(server.panes(server.sessions().front().window)->at(1).columns, 44);
    EXPECT_EQ(server.window(server.sessions().front().window)->pane, PaneId{1});
    const auto revision = server.window(server.sessions().front().window)->revision;
    for (const auto &arguments :
         std::vector<std::vector<std::string>>{{"resize-pane", "-R", "-Z", "-t", "%1"},
                                               {"resize-pane", "-R", "-L", "-t", "%1"},
                                               {"resize-pane", "-R", "-t", "%1", "4294967296"},
                                               {"resize-pane", "-R", "-t", "%1", "2147483648"},
                                               {"resize-pane", "-R", "-t", "%1", "0"},
                                               {"resize-pane", "-R", "-t", "%1", "-1"},
                                               {"resize-pane", "-R", "-t", "%1", "5oops"}}) {
        EXPECT_FALSE(command_catalog->invoke(invocation, arguments));
        EXPECT_EQ(server.window(server.sessions().front().window)->revision, revision);
    }
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
