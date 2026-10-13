#include "client/client_key_input.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "commands/command_registry.hpp"
#include "protocol/native/framing.hpp"
#include "protocol/native/operation_registry.hpp"
#include "server/api_catalogs.hpp"
#include "session/session_options.hpp"
#include "window/window_options.hpp"

#include <tmux_cxx/server/server.hpp>
#include <tmux_cxx/session/operations/set_session_option.hpp>
#include <tmux_cxx/window/operations/read_window_options.hpp>
#include <tmux_cxx/window/operations/set_window_option.hpp>

#include <algorithm>
#include <array>
#include <bit>
#include <gtest/gtest.h>
#include <optional>
#include <string_view>
#include <vector>

namespace tmux_cxx {
/** @brief Window mode choices reject incompatible values while retaining the exact tmux names. */
TEST(OptionTable, WindowModeKeysUseClosedChoiceValues) {
    auto defaults = options::OptionTable::defaults(window_options::catalog());
    options::OptionTable overrides;
    EXPECT_EQ(window_options::effective_mode_keys(overrides, defaults), WindowModeKeys::emacs);
    ASSERT_EQ(window_mode_keys.size(), 2U);
    EXPECT_EQ(window_mode_keys[0].name, "emacs");
    EXPECT_EQ(window_mode_keys[1].name, "vi");
    const auto parsed = window_options::catalog().parse("mode-keys", "vi");
    ASSERT_TRUE(parsed);
    ASSERT_TRUE(overrides.set(*parsed));
    EXPECT_EQ(window_options::effective_mode_keys(overrides, defaults), WindowModeKeys::vi);
    EXPECT_FALSE(window_options::catalog().parse("mode-keys", "VI"));
    EXPECT_FALSE(window_options::catalog().parse("mode-keys", "None"));
    EXPECT_FALSE(window_options::catalog().parse("prefix", "C-a"));
    const auto invalid_mode_keys = std::bit_cast<WindowModeKeys>(std::uint32_t{2});
    const options::ParsedOption invalid{&window_options::mode_keys, invalid_mode_keys};
    EXPECT_FALSE(overrides.set(invalid));
    EXPECT_FALSE(valid_window_mode_keys(invalid_mode_keys));
    EXPECT_EQ(window_options::effective_mode_keys(overrides, defaults), WindowModeKeys::vi);
}

/** @brief Startup editor defaults preserve VISUAL precedence and tmux's literal basename match. */
TEST(WindowOptions, EditorDefaultsFollowPinnedTmuxSource) {
    struct EditorCase {
        std::optional<std::string_view> visual;
        std::optional<std::string_view> editor;
        WindowModeKeys expected;
    };
    const std::array cases{
        EditorCase{std::nullopt, std::nullopt, WindowModeKeys::emacs},
        EditorCase{std::nullopt, "/usr/bin/nvim", WindowModeKeys::vi},
        EditorCase{"", "vi", WindowModeKeys::emacs},
        EditorCase{"emacs", "vi", WindowModeKeys::emacs},
        EditorCase{"/usr/bin/vim", "emacs", WindowModeKeys::vi},
        EditorCase{"/vi/bin/emacs", std::nullopt, WindowModeKeys::emacs},
        EditorCase{"VIM", std::nullopt, WindowModeKeys::emacs},
        EditorCase{"evil", std::nullopt, WindowModeKeys::vi},
    };
    for (const auto &test : cases) {
        EXPECT_EQ(window_options::mode_keys_from_editors(test.visual, test.editor), test.expected);
    }
}

/** @brief Existing and new windows inherit live defaults while explicit equal assignments pin
 * local values. */
TEST(WindowOptions, GlobalDefaultsRemainLiveUntilLocallyPinned) {
    Server server;
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "emacs"));
    const auto inherited = server.create_session("inherited-window", "/bin/cat");
    const auto pinned = server.create_session("pinned-window", "/bin/cat");
    ASSERT_TRUE(inherited);
    ASSERT_TRUE(pinned);
    const auto before_pin = server.window_options(pinned->window);
    ASSERT_TRUE(before_pin);
    EXPECT_EQ(before_pin->server_instance, server.server_instance());
    EXPECT_EQ(before_pin->window, pinned->window);
    EXPECT_EQ(before_pin->mode_keys, WindowModeKeys::emacs);

    ASSERT_TRUE(server.set_window_option(pinned->window, "mode-keys", "emacs"));
    const auto after_pin = server.window_options(pinned->window);
    ASSERT_TRUE(after_pin);
    EXPECT_GT(after_pin->revision, before_pin->revision);
    ASSERT_TRUE(server.set_window_option(pinned->window, "mode-keys", "emacs"));
    EXPECT_EQ(server.window_options(pinned->window)->revision, after_pin->revision);
    const auto before_global = server.window_options(GlobalWindowOptions{});
    const auto before_inherited = server.window_options(inherited->window);
    ASSERT_TRUE(before_global);
    ASSERT_TRUE(before_inherited);
    EXPECT_FALSE(before_global->window);

    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "vi"));
    const auto after_global = server.window_options(GlobalWindowOptions{});
    ASSERT_TRUE(after_global);
    EXPECT_EQ(after_global->revision, before_global->revision + 1);
    EXPECT_EQ(server.window_options(inherited->window)->revision, after_global->revision);
    EXPECT_GT(server.window_options(inherited->window)->revision, before_inherited->revision);
    EXPECT_EQ(server.window_mode_keys(inherited->window), WindowModeKeys::vi);
    EXPECT_EQ(server.window_mode_keys(pinned->window), WindowModeKeys::emacs);
    EXPECT_EQ(server.window_options(pinned->window)->revision, after_pin->revision);
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "vi"));
    EXPECT_EQ(server.window_options(GlobalWindowOptions{})->revision, after_global->revision);

    const auto later = server.create_window(pinned->id, 7, "later-window", "/bin/cat");
    ASSERT_TRUE(later);
    EXPECT_EQ(server.window_mode_keys(later->window), WindowModeKeys::vi);
    ASSERT_TRUE(server.set_window_option(later->window, "mode-keys", "emacs"));
    EXPECT_EQ(server.window_mode_keys(inherited->window), WindowModeKeys::vi);
}

/** @brief A shared window retains its option override through linked-session and membership
 * lifetimes. */
TEST(WindowOptions, LinkedWindowsShareOverridesBeyondOriginalMembership) {
    Server server;
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "emacs"));
    const auto original = server.create_session("original-window", "/bin/cat");
    const auto second = server.create_session("second-window", "/bin/cat");
    ASSERT_TRUE(original);
    ASSERT_TRUE(second);
    const auto linked = server.link_window(original->window, second->id, 7);
    ASSERT_TRUE(linked);
    ASSERT_TRUE(server.set_window_option(linked->window, "mode-keys", "vi"));
    EXPECT_EQ(server.window_mode_keys(original->window), WindowModeKeys::vi);
    EXPECT_EQ(server.window_mode_keys(second->window), WindowModeKeys::emacs);
    ASSERT_TRUE(server.unlink_window(original->link));
    EXPECT_EQ(server.window_mode_keys(linked->window), WindowModeKeys::vi);
    ASSERT_TRUE(server.unlink_window(linked->id));
    const auto removed = server.window_options(original->window);
    ASSERT_FALSE(removed);
    EXPECT_EQ(removed.error().code, TmuxErrorCode::not_found);
}

/** @brief Unsupported values, scopes, and stale targets preserve option state and metadata
 * revisions. */
TEST(WindowOptions, InvalidAssignmentsRejectBeforeEffects) {
    Server server;
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "emacs"));
    const auto session = server.create_session("option-validation", "/bin/cat");
    ASSERT_TRUE(session);
    const auto before_global = server.window_options(GlobalWindowOptions{});
    const auto before_window = server.window_options(session->window);
    ASSERT_TRUE(before_global);
    ASSERT_TRUE(before_window);
    EXPECT_FALSE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "2"));
    EXPECT_FALSE(server.set_window_option(session->window, "prefix", "C-a"));
    EXPECT_FALSE(server.set_session_option(session->id, "mode-keys", "vi"));
    const auto zero = server.set_window_option(WindowId{}, "mode-keys", "vi");
    ASSERT_FALSE(zero);
    EXPECT_EQ(zero.error().code, TmuxErrorCode::invalid_argument);
    const auto stale = server.set_window_option(WindowId{999999}, "mode-keys", "vi");
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
    EXPECT_EQ(server.window_options(GlobalWindowOptions{})->revision, before_global->revision);
    EXPECT_EQ(server.window_options(session->window)->revision, before_window->revision);
    EXPECT_EQ(server.window_mode_keys(session->window), WindowModeKeys::emacs);
}

/** @brief Native target tags, truncation, and trailing bytes fail before option assignment. */
TEST(NativeProtocol, WindowOptionRequestsValidateBeforeMutation) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<SetWindowOption>());
    ASSERT_TRUE(operations.add<ReadWindowOptions>());
    const auto catalog = std::move(operations).freeze();
    ASSERT_TRUE(catalog);
    Server server;
    ASSERT_TRUE(server.set_window_option(GlobalWindowOptions{}, "mode-keys", "emacs"));
    const auto session = server.create_session("native-window-option", "/bin/cat");
    ASSERT_TRUE(session);
    const auto before = server.window_options(session->window);
    ASSERT_TRUE(before);
    for (const auto tag : {std::uint16_t{2}, std::uint16_t{1}}) {
        protocol::native::NativePayloadWriter malformed;
        malformed.u16(tag);
        const auto rejected =
            catalog->invoke(SetWindowOption::description.id, server, malformed.bytes);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::protocol);
    }
    protocol::native::NativePayloadWriter trailing;
    trailing.u16(1);
    trailing.u64(session->window.value);
    trailing.string("mode-keys");
    trailing.string("vi");
    trailing.u16(0);
    EXPECT_FALSE(catalog->invoke(SetWindowOption::description.id, server, trailing.bytes));
    EXPECT_EQ(server.window_options(session->window)->revision, before->revision);

    protocol::native::NativePayloadWriter assignment;
    assignment.u16(1);
    assignment.u64(session->window.value);
    assignment.string("mode-keys");
    assignment.string("vi");
    ASSERT_TRUE(catalog->invoke(SetWindowOption::description.id, server, assignment.bytes));
    protocol::native::NativePayloadWriter query;
    query.u16(1);
    query.u64(session->window.value);
    const auto reply = catalog->invoke(ReadWindowOptions::description.id, server, query.bytes);
    ASSERT_TRUE(reply);
    ASSERT_TRUE(std::holds_alternative<protocol::native::Bytes>(*reply));
    protocol::native::NativePayloadReader reader(std::get<protocol::native::Bytes>(*reply));
    EXPECT_EQ(reader.string(), server.server_instance());
    EXPECT_EQ(reader.u16(), 1);
    EXPECT_EQ(reader.u64(), session->window.value);
    EXPECT_EQ(reader.u32(), static_cast<std::uint32_t>(WindowModeKeys::vi));
    EXPECT_EQ(reader.u64(), server.window_options(session->window)->revision);
    EXPECT_TRUE(reader.finished());
}

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
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    const auto *command_catalog = &catalogs->stock_commands;

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

    ClientKeyInput input;
    constexpr std::array<char, 2> old_prefix{2, '%'};
    ASSERT_TRUE(input.append({old_prefix.data(), old_prefix.size()}));
    auto effect = input.route_pending(server, *client,
                                      {session->id, session->link, session->window, session->pane},
                                      *prefix_keys, 262144, catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(server.panes(session->window)->size(), 1U);

    constexpr std::array<char, 2> primary_prefix{1, '%'};
    ASSERT_TRUE(input.append({primary_prefix.data(), primary_prefix.size()}));
    effect = input.route_pending(server, *client,
                                 {session->id, session->link, session->window, session->pane},
                                 *prefix_keys, 262144, catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, key_table::KeyBindingEffect::client_selection_changed);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2U);
    const auto active = std::ranges::find(*panes, true, &PaneSnapshot::active);
    ASSERT_NE(active, panes->end());

    constexpr std::array<char, 2> secondary_prefix{24, '"'};
    ASSERT_TRUE(input.append({secondary_prefix.data(), secondary_prefix.size()}));
    effect = input.route_pending(server, *client,
                                 {session->id, session->link, session->window, active->id},
                                 *prefix_keys, 262144, catalogs->key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, key_table::KeyBindingEffect::client_selection_changed);
    EXPECT_EQ(server.panes(session->window)->size(), 3U);
}

/** @brief A targeted session option overrides its inherited global value without changing peers.
 */
TEST(CommandRegistry, SetOptionTargetsOneSessionWithoutChangingInheritedPeers) {
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    const auto *command_catalog = &catalogs->stock_commands;

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
