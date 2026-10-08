#include "client/client_key_input.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "commands/command_registry.hpp"
#include "protocol/native/framing.hpp"
#include "server/api_catalogs.hpp"
#include "session/register_commands.hpp"
#include "session/register_operations.hpp"
#include "session/session_options.hpp"

#include <tmux_cxx/server/server.hpp>
#include <tmux_cxx/session/operations/set_session_option.hpp>

#include <algorithm>
#include <array>
#include <gtest/gtest.h>
#include <vector>

namespace tmux_cxx {
/** @brief Session prefix options inherit global values, permit overrides, and reject invalid text.
 */
TEST(OptionTable, SessionPrefixKeysInheritOverrideAndValidate) {
    auto global_options = session_options::global_defaults();
    options::OptionTable session_overrides;

    auto inherited = session_options::prefix_keys(session_overrides, global_options);
    ASSERT_TRUE(inherited);
    EXPECT_EQ(inherited->prefix, key_table::control_key('b'));
    EXPECT_FALSE(inherited->prefix2);

    auto global_prefix = session_options::catalog().parse("prefix", "C-a");
    ASSERT_TRUE(global_prefix);
    auto global_changed = global_options.set(*global_prefix);
    ASSERT_TRUE(global_changed);
    EXPECT_TRUE(*global_changed);
    inherited = session_options::prefix_keys(session_overrides, global_options);
    ASSERT_TRUE(inherited);
    EXPECT_EQ(inherited->prefix, key_table::control_key('a'));

    auto local_prefix = session_options::catalog().parse("prefix", "C-x");
    ASSERT_TRUE(local_prefix);
    ASSERT_TRUE(session_overrides.set(*local_prefix));
    auto overridden = session_options::prefix_keys(session_overrides, global_options);
    ASSERT_TRUE(overridden);
    EXPECT_EQ(overridden->prefix, key_table::control_key('x'));
    EXPECT_TRUE(session_overrides.unset(session_options::prefix));
    inherited = session_options::prefix_keys(session_overrides, global_options);
    ASSERT_TRUE(inherited);
    EXPECT_EQ(inherited->prefix, key_table::control_key('a'));

    EXPECT_FALSE(session_options::catalog().parse("prefix", "M-a"));
    EXPECT_FALSE(session_options::catalog().parse("status", "on"));
}

/** @brief Native session-option requests distinguish global defaults from resolved session
 * overrides. */
TEST(NativeProtocol, SetSessionOptionSelectsOneOptionTable) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<SetSessionOption>());
    auto catalog = std::move(operations).freeze();
    ASSERT_TRUE(catalog);

    Server server;
    const auto targeted = server.create_session("native-target", "/bin/cat");
    const auto inherited = server.create_session("native-inherited", "/bin/cat");
    ASSERT_TRUE(targeted);
    ASSERT_TRUE(inherited);

    protocol::native::NativePayloadWriter global;
    global.string("prefix");
    global.string("C-a");
    ASSERT_TRUE(catalog->invoke(SetSessionOption::description.id, server, global.bytes));

    protocol::native::NativePayloadWriter local;
    local.string("prefix");
    local.string("C-x");
    local.u64(targeted->id.value);
    ASSERT_TRUE(catalog->invoke(SetSessionOption::description.id, server, local.bytes));

    const auto targeted_keys = server.session_prefix_keys(targeted->id);
    const auto inherited_keys = server.session_prefix_keys(inherited->id);
    ASSERT_TRUE(targeted_keys);
    ASSERT_TRUE(inherited_keys);
    EXPECT_EQ(targeted_keys->prefix, key_table::control_key('x'));
    EXPECT_EQ(inherited_keys->prefix, key_table::control_key('a'));

    protocol::native::NativePayloadWriter malformed;
    malformed.string("prefix");
    malformed.string("C-b");
    malformed.u16(1);
    const auto rejected =
        catalog->invoke(SetSessionOption::description.id, server, malformed.bytes);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::protocol);
    EXPECT_EQ(server.session_prefix_keys(targeted->id)->prefix, key_table::control_key('x'));
}

/** @brief Registered global prefix changes drive key routing without event-loop command branches.
 */
TEST(CommandRegistry, SetOptionChangesInteractivePrefixThroughTypedSessionOperation) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(register_session_operations(operations));
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_session_commands(command_registry));
    auto command_catalog = std::move(command_registry).freeze(*operation_catalog);
    ASSERT_TRUE(command_catalog);

    Server server;
    const auto session = server.create_session("prefix-options", "/bin/cat");
    ASSERT_TRUE(session);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};

    std::vector<std::string> set_prefix{"set-option", "-g", "prefix", "C-a"};
    ASSERT_TRUE(command_catalog->invoke(invocation, set_prefix));
    std::vector<std::string> set_prefix2{"set", "-g", "prefix2", "C-x"};
    ASSERT_TRUE(command_catalog->invoke(invocation, set_prefix2));
    const auto prefix_keys = server.session_prefix_keys(session->id);
    ASSERT_TRUE(prefix_keys);
    EXPECT_EQ(prefix_keys->prefix, key_table::control_key('a'));
    EXPECT_EQ(prefix_keys->prefix2, key_table::control_key('x'));

    std::vector<std::string> invalid_key{"set-option", "-g", "prefix", "M-a"};
    EXPECT_FALSE(command_catalog->invoke(invocation, invalid_key));
    std::vector<std::string> invalid_scope{"set-option", "-w", "prefix", "C-b"};
    EXPECT_FALSE(command_catalog->invoke(invocation, invalid_scope));
    std::vector<std::string> unknown_option{"set-option", "-g", "status", "on"};
    EXPECT_FALSE(command_catalog->invoke(invocation, unknown_option));
    EXPECT_EQ(server.session_prefix_keys(session->id)->prefix, key_table::control_key('a'));

    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    ClientKeyInput input;
    constexpr std::array<char, 2> old_prefix{2, '%'};
    ASSERT_TRUE(input.append({old_prefix.data(), old_prefix.size()}));
    auto effect = input.route_pending(server, *client, session->window, session->pane, *prefix_keys,
                                      262144, catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(server.panes(session->window)->size(), 1U);

    constexpr std::array<char, 2> primary_prefix{1, '%'};
    ASSERT_TRUE(input.append({primary_prefix.data(), primary_prefix.size()}));
    effect = input.route_pending(server, *client, session->window, session->pane, *prefix_keys,
                                 262144, catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, key_table::KeyBindingEffect::selected_pane_changed);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2U);
    const auto active = std::ranges::find(*panes, true, &PaneSnapshot::active);
    ASSERT_NE(active, panes->end());

    constexpr std::array<char, 2> secondary_prefix{24, '"'};
    ASSERT_TRUE(input.append({secondary_prefix.data(), secondary_prefix.size()}));
    effect = input.route_pending(server, *client, session->window, active->id, *prefix_keys, 262144,
                                 catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, key_table::KeyBindingEffect::selected_pane_changed);
    EXPECT_EQ(server.panes(session->window)->size(), 3U);
}

/** @brief A targeted session option overrides its inherited global value without changing peers.
 */
TEST(CommandRegistry, SetOptionTargetsOneSessionWithoutChangingInheritedPeers) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(register_session_operations(operations));
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    commands::CommandRegistry command_registry;
    ASSERT_TRUE(register_session_commands(command_registry));
    auto command_catalog = std::move(command_registry).freeze(*operation_catalog);
    ASSERT_TRUE(command_catalog);

    Server server;
    const auto targeted = server.create_session("targeted", "/bin/cat");
    const auto inherited = server.create_session("inherited", "/bin/cat");
    ASSERT_TRUE(targeted);
    ASSERT_TRUE(inherited);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    commands::CommandInvocation invocation{server, *client};

    std::vector<std::string> set_global{"set-option", "-g", "prefix", "C-a"};
    ASSERT_TRUE(command_catalog->invoke(invocation, set_global));
    std::vector<std::string> set_target{"set-option", "-t", "targeted", "prefix", "C-x"};
    ASSERT_TRUE(command_catalog->invoke(invocation, set_target));

    auto targeted_keys = server.session_prefix_keys(targeted->id);
    auto inherited_keys = server.session_prefix_keys(inherited->id);
    ASSERT_TRUE(targeted_keys);
    ASSERT_TRUE(inherited_keys);
    EXPECT_EQ(targeted_keys->prefix, key_table::control_key('x'));
    EXPECT_EQ(inherited_keys->prefix, key_table::control_key('a'));

    std::vector<std::string> missing_target{"set-option", "-t", "missing", "prefix", "C-b"};
    EXPECT_FALSE(command_catalog->invoke(invocation, missing_target));
    std::vector<std::string> invalid_value{"set-option", "-t", "targeted", "prefix", "M-a"};
    EXPECT_FALSE(command_catalog->invoke(invocation, invalid_value));
    targeted_keys = server.session_prefix_keys(targeted->id);
    inherited_keys = server.session_prefix_keys(inherited->id);
    ASSERT_TRUE(targeted_keys);
    ASSERT_TRUE(inherited_keys);
    EXPECT_EQ(targeted_keys->prefix, key_table::control_key('x'));
    EXPECT_EQ(inherited_keys->prefix, key_table::control_key('a'));
}
} // namespace tmux_cxx
