#include "key_table/key_binding.hpp"
#include "server/api_catalogs.hpp"

#include <tmux_cxx/server/server.hpp>

#include <gtest/gtest.h>
#include <string>
#include <vector>

namespace tmux_cxx {
/** @brief Stock copy syntax rejects unsupported forms before selection or display freshness
 * changes. */
TEST(CopyModeCommands, CatalogActionsRejectMalformedSyntaxBeforeEffects) {
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    Server server;
    const auto session = server.create_session("copy-commands", "/bin/cat");
    ASSERT_TRUE(session);
    commands::CommandInvocation invocation{server, ClientId{1}};
    const auto target = "%" + std::to_string(session->pane.value);
    ASSERT_TRUE(catalogs->stock_commands.invoke(
        invocation, std::vector<std::string>{"copy-mode", "-t", target}));
    ASSERT_TRUE(catalogs->stock_commands.invoke(
        invocation, std::vector<std::string>{"send-keys", "-X", "-t", target, "begin-selection"}));
    const auto selected = server.copy_mode(session->pane);
    ASSERT_TRUE(selected);
    ASSERT_TRUE(selected->mode);
    ASSERT_TRUE(selected->mode->selection);
    for (const auto &arguments : std::vector<std::vector<std::string>>{
             {"copy-mode"},
             {"copy-mode", "-u", "-t", target},
             {"send-keys", "-X", "cursor-left"},
             {"send-keys", "-X", "-N", "0", "-t", target, "cursor-right"},
             {"send-keys", "-X", "-N", "4294967296", "-t", target, "cursor-right"},
             {"send-keys", "-XX", "-t", target, "cursor-right"},
             {"send-keys", "-X", "-t", target, "copy-pipe-and-cancel"},
             {"send-keys", "-X", "-t", target, "cursor-right", "extra"}}) {
        EXPECT_FALSE(catalogs->stock_commands.invoke(invocation, arguments));
        const auto retained = server.copy_mode(session->pane);
        ASSERT_TRUE(retained);
        ASSERT_TRUE(retained->mode);
        EXPECT_EQ(retained->display_revision, selected->display_revision);
        ASSERT_TRUE(retained->mode->selection);
        EXPECT_EQ(retained->mode->selection->anchor, selected->mode->selection->anchor);
    }
    ASSERT_TRUE(catalogs->stock_commands.invoke(
        invocation, std::vector<std::string>{"send", "-X", "-t", target, "cancel"}));
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    EXPECT_FALSE(catalogs->stock_commands.invoke(
        invocation, std::vector<std::string>{"send", "-X", "-t", target, "cancel"}));
}

/** @brief Operation-backed mode keys retain vi Escape selection semantics and emacs cancellation.
 */
TEST(CopyModeKeyBindings, BasicDefaultsInvokeSharedPaneOperations) {
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    Server server;
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "emacs"));
    const auto session = server.create_session("copy-keys", "/bin/cat");
    ASSERT_TRUE(session);
    key_table::KeyBindingInvocation invocation{
        server,
        ClientId{1},
        {session->id, session->link, session->window, session->pane},
        std::nullopt};
    auto entered = catalogs->key_tables.invoke("prefix", key_table::KeyCode{'['}, invocation);
    ASSERT_TRUE(entered);
    ASSERT_TRUE(*entered);
    EXPECT_EQ(**entered, key_table::KeyBindingEffect::pane_mode_changed);
    ASSERT_TRUE(server.copy_mode(session->pane)->mode);
    auto begin = catalogs->key_tables.invoke("copy-mode", key_table::KeyCode{0}, invocation);
    ASSERT_TRUE(begin);
    ASSERT_TRUE(*begin);
    ASSERT_TRUE(server.copy_mode(session->pane)->mode->selection);
    auto cleared = catalogs->key_tables.invoke("copy-mode-vi", key_table::escape_key, invocation);
    ASSERT_TRUE(cleared);
    ASSERT_TRUE(*cleared);
    EXPECT_EQ(**cleared, key_table::KeyBindingEffect::client_selection_unchanged);
    ASSERT_TRUE(server.copy_mode(session->pane)->mode);
    EXPECT_FALSE(server.copy_mode(session->pane)->mode->selection);
    EXPECT_FALSE(catalogs->key_tables.contains("copy-mode-vi", key_table::KeyCode{'y'}));
    EXPECT_FALSE(catalogs->key_tables.contains("copy-mode", key_table::KeyCode{13}));
    auto cancelled = catalogs->key_tables.invoke("copy-mode", key_table::escape_key, invocation);
    ASSERT_TRUE(cancelled);
    ASSERT_TRUE(*cancelled);
    EXPECT_EQ(**cancelled, key_table::KeyBindingEffect::pane_mode_changed);
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    ASSERT_TRUE(catalogs->key_tables.invoke("prefix", key_table::KeyCode{'['}, invocation));
    auto copied = catalogs->key_tables.invoke("copy-mode-vi", key_table::KeyCode{13}, invocation);
    ASSERT_TRUE(copied);
    ASSERT_TRUE(*copied);
    EXPECT_EQ(**copied, key_table::KeyBindingEffect::pane_mode_changed);
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    EXPECT_TRUE(server.paste_buffers()->empty());
}
} // namespace tmux_cxx
