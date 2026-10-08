#include "client/client_key_input.hpp"
#include "client/operations/detach_client_key_binding.hpp"
#include "key_table/key_binding_registry.hpp"
#include "pane/operations/select_next_pane_key_binding.hpp"
#include "protocol/native/operation_registry.hpp"
#include "server/api_catalogs.hpp"

#include <tmux_cxx/client/operations/detach_client.hpp>
#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <array>
#include <gtest/gtest.h>
#include <utility>

namespace tmux_cxx::key_table {
namespace {
constexpr SessionPrefixKeys default_prefixes{
    control_key('b'), std::nullopt}; ///< Tmux defaults used by isolated input-routing tests.

/** @brief Provide a binding whose undeclared operation must prevent catalog construction. */
struct UnknownOperationBinding {
    /** @brief Reference an operation identity absent from the test catalog. */
    static constexpr KeyBindingDescription description{"prefix", KeyCode{'x'}, "unknown-operation",
                                                       protocol::native::OperationId{999}};
    /** @brief Remain unreachable because startup rejects the operation reference first. */
    static Result<KeyBindingEffect> execute(KeyBindingInvocation &) {
        return KeyBindingEffect::selected_pane_unchanged;
    }
};

/** @brief Return a distinctive effect when prefix fallback reaches a root-table binding. */
struct RootFallbackBinding {
    /** @brief Declare a synthetic root binding backed by a registered test operation. */
    static constexpr KeyBindingDescription description{"root", KeyCode{'x'}, "detach-client",
                                                       DetachClient::description.id};
    /** @brief Mark root fallback without mutating the otherwise empty test server. */
    static Result<KeyBindingEffect> execute(KeyBindingInvocation &) {
        return KeyBindingEffect::selected_pane_changed;
    }
};

int retryable_binding_calls = 0; ///< Count synthetic binding attempts within the owning test.

/** @brief Return backpressure once so input routing must preserve the selected prefix table. */
struct RetryablePrefixBinding {
    /** @brief Declare a synthetic prefix binding backed by a registered test operation. */
    static constexpr KeyBindingDescription description{"prefix", KeyCode{'r'}, "detach-client",
                                                       DetachClient::description.id};
    /** @brief Reject the first attempt before effects and accept the retried binding. */
    static Result<KeyBindingEffect> execute(KeyBindingInvocation &) {
        ++retryable_binding_calls;
        if (retryable_binding_calls == 1) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure, "synthetic key binding backpressure"});
        }
        return KeyBindingEffect::selected_pane_unchanged;
    }
};
} // namespace

/** @brief Duplicate, late, and undeclared-operation bindings fail before runtime lookup. */
TEST(KeyBindingRegistry, StartupValidationPrecedesImmutableLookup) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);

    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<DetachClientKeyBinding>());
    EXPECT_FALSE(registry.add<DetachClientKeyBinding>());
    auto key_tables = std::move(registry).freeze(*operation_catalog);
    ASSERT_TRUE(key_tables);
    EXPECT_FALSE(registry.add<DetachClientKeyBinding>());

    KeyBindingRegistry invalid;
    ASSERT_TRUE(invalid.add<UnknownOperationBinding>());
    const auto rejected = std::move(invalid).freeze(*operation_catalog);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
}

/** @brief An unbound prefix key retries a matching root binding before consuming the key. */
TEST(ClientKeyInput, UnmatchedPrefixBindingRetriesRootTable) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<RootFallbackBinding>());
    auto key_tables = std::move(registry).freeze(*operation_catalog);
    ASSERT_TRUE(key_tables);

    ClientKeyInput input;
    ASSERT_TRUE(input.append("\x02x"));
    Server server;
    const auto effect = input.route_pending(server, ClientId{1}, WindowId{1}, PaneId{1},
                                            default_prefixes, 0, *key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, KeyBindingEffect::selected_pane_changed);
}

/** @brief Selecting next in a one-pane window reports that the selected pane stayed unchanged. */
TEST(ClientKeyInput, NextPaneBindingReportsUnchangedForSinglePane) {
    Server server;
    const auto session = server.create_session("one-pane", "/bin/cat");
    ASSERT_TRUE(session);
    KeyBindingInvocation invocation{server, ClientId{1}, session->window, session->pane,
                                    default_prefixes.prefix};

    const auto effect = SelectNextPaneKeyBinding::execute(invocation);

    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, KeyBindingEffect::selected_pane_unchanged);
}

/** @brief Binding backpressure leaves the command key and prefix-table selection retryable. */
TEST(ClientKeyInput, BindingBackpressurePreservesPrefixTable) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<RetryablePrefixBinding>());
    auto key_tables = std::move(registry).freeze(*operation_catalog);
    ASSERT_TRUE(key_tables);
    ClientKeyInput input;
    ASSERT_TRUE(input.append("\x02r"));
    Server server;
    retryable_binding_calls = 0;

    const auto deferred = input.route_pending(server, ClientId{1}, WindowId{1}, PaneId{1},
                                              default_prefixes, 0, *key_tables);
    ASSERT_TRUE(deferred);
    EXPECT_EQ(*deferred, KeyBindingEffect::selected_pane_unchanged);
    EXPECT_EQ(retryable_binding_calls, 1);

    const auto retried = input.route_pending(server, ClientId{1}, WindowId{1}, PaneId{1},
                                             default_prefixes, 0, *key_tables);
    ASSERT_TRUE(retried);
    EXPECT_EQ(*retried, KeyBindingEffect::selected_pane_unchanged);
    EXPECT_EQ(retryable_binding_calls, 2);
}

/** @brief Default percent and quote bindings split the selected pane through `SplitPane`. */
TEST(ClientKeyInput, DefaultSplitBindingsUseTypedPaneOperation) {
    Server server;
    const auto session = server.create_session("key-splits", "/bin/cat");
    ASSERT_TRUE(session);
    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    ClientKeyInput input;
    constexpr std::array<char, 2> left_right{2, '%'};
    ASSERT_TRUE(input.append({left_right.data(), left_right.size()}));

    const auto first_split =
        input.route_pending(server, ClientId{1}, session->window, session->pane, default_prefixes,
                            262144, catalogs->key_tables);

    ASSERT_TRUE(first_split);
    EXPECT_EQ(*first_split, KeyBindingEffect::selected_pane_changed);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2U);
    const auto active_after_first = std::ranges::find(*panes, true, &PaneSnapshot::active);
    ASSERT_NE(active_after_first, panes->end());
    EXPECT_NE(active_after_first->id, session->pane);
    constexpr std::array<char, 2> top_bottom{2, '"'};
    ASSERT_TRUE(input.append({top_bottom.data(), top_bottom.size()}));

    const auto second_split =
        input.route_pending(server, ClientId{1}, session->window, active_after_first->id,
                            default_prefixes, 262144, catalogs->key_tables);

    ASSERT_TRUE(second_split);
    EXPECT_EQ(*second_split, KeyBindingEffect::selected_pane_changed);
    panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    EXPECT_EQ(panes->size(), 3U);
    EXPECT_EQ(std::ranges::count(*panes, true, &PaneSnapshot::active), 1);
}
} // namespace tmux_cxx::key_table
