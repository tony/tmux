#include "client/client_key_input.hpp"
#include "client/operations/detach_client_key_binding.hpp"
#include "key_table/key_binding_registry.hpp"
#include "pane/operations/select_next_pane_key_binding.hpp"
#include "protocol/native/operation_registry.hpp"
#include "server/api_catalogs.hpp"
#include "window_link/operations/select_window_key_binding.hpp"
#include "window_link/register_key_bindings.hpp"
#include <tmux_cxx/window_link/operations/next_window.hpp>
#include <tmux_cxx/window_link/operations/previous_window.hpp>
#include <tmux_cxx/window_link/operations/select_window.hpp>

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
        return KeyBindingEffect::client_selection_unchanged;
    }
};

/** @brief Return a distinctive effect when prefix fallback reaches a root-table binding. */
struct RootFallbackBinding {
    /** @brief Declare a synthetic root binding backed by a registered test operation. */
    static constexpr KeyBindingDescription description{"root", KeyCode{'x'}, "detach-client",
                                                       DetachClient::description.id};
    /** @brief Mark root fallback without mutating the otherwise empty test server. */
    static Result<KeyBindingEffect> execute(KeyBindingInvocation &) {
        return KeyBindingEffect::client_selection_changed;
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
        return KeyBindingEffect::client_selection_unchanged;
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
    const auto effect = input.route_pending(server, ClientId{1},
                                            {SessionId{1}, WindowLinkId{1}, WindowId{1}, PaneId{1}},
                                            default_prefixes, 0, *key_tables);
    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, KeyBindingEffect::client_selection_changed);
}

/** @brief Selecting next in a one-pane window reports that the selected pane stayed unchanged. */
TEST(ClientKeyInput, NextPaneBindingReportsUnchangedForSinglePane) {
    Server server;
    const auto session = server.create_session("one-pane", "/bin/cat");
    ASSERT_TRUE(session);
    KeyBindingInvocation invocation{server,
                                    ClientId{1},
                                    {session->id, session->link, session->window, session->pane},
                                    default_prefixes.prefix};

    const auto effect = SelectNextPaneKeyBinding::execute(invocation);

    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, KeyBindingEffect::client_selection_unchanged);
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

    const auto deferred = input.route_pending(
        server, ClientId{1}, {SessionId{1}, WindowLinkId{1}, WindowId{1}, PaneId{1}},
        default_prefixes, 0, *key_tables);
    ASSERT_TRUE(deferred);
    EXPECT_EQ(*deferred, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(retryable_binding_calls, 1);
    EXPECT_FALSE(input.routing_deferred());

    const auto retried = input.route_pending(
        server, ClientId{1}, {SessionId{1}, WindowLinkId{1}, WindowId{1}, PaneId{1}},
        default_prefixes, 0, *key_tables);
    ASSERT_TRUE(retried);
    EXPECT_EQ(*retried, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(retryable_binding_calls, 2);
}

/** @brief Input-producing bindings preserve coalesced ordinary bytes until capacity is resolved
 * again. */
TEST(ClientKeyInput, ProgramInputBindingsRetainSuffixAtCapacity) {
    for (const bool paste : {true, false}) {
        SCOPED_TRACE(paste ? "paste-buffer" : "send-prefix");
        Server server;
        const auto session = server.create_session("input-capacity", "/bin/cat");
        ASSERT_TRUE(session);
        const auto catalogs = build_server_api_catalogs();
        ASSERT_TRUE(catalogs);
        const std::string retained(262143, 'x');
        if (paste) {
            ASSERT_TRUE(server.add_paste_buffer(retained));
        } else {
            ASSERT_TRUE(server.set_paste_buffer("prefill", retained));
            ASSERT_TRUE(server.paste_buffer_into_pane("prefill", session->pane));
        }
        ClientKeyInput input;
        const auto empty_capacity = input.read_capacity();
        ASSERT_TRUE(input.append(paste ? "\x02]xx" : "\x02\x02xx"));
        const ClientSelection selection{session->id, session->link, session->window, session->pane};

        const auto admitted = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                                  paste ? 262144 : 1, catalogs->key_tables);

        EXPECT_TRUE(admitted);
        if (!admitted) {
            continue;
        }
        EXPECT_EQ(*admitted, KeyBindingEffect::pane_program_input_changed);
        EXPECT_EQ(input.read_capacity(), empty_capacity - 2);
        const auto resumed = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                                 paste ? 1 : 0, catalogs->key_tables);
        ASSERT_TRUE(resumed);
        EXPECT_EQ(input.read_capacity(), empty_capacity - (paste ? 1 : 2));
        const auto full = server.send_pane_input(session->pane, "x");
        ASSERT_FALSE(full);
        EXPECT_EQ(full.error().code, TmuxErrorCode::backpressure);
    }
}

/** @brief Resolving capacity between paste bindings preserves the same 64-decision turn limit. */
TEST(ClientKeyInput, ProgramInputBindingsShareRoutingAllowance) {
    Server server;
    const auto session = server.create_session("input-fairness", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(server.add_paste_buffer("x"));
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    std::string burst;
    for (unsigned int index = 0; index < 64; ++index) {
        burst.append("\x02]");
    }
    ASSERT_TRUE(input.append(burst));
    const ClientSelection selection{session->id, session->link, session->window, session->pane};

    for (unsigned int index = 0; index < 32; ++index) {
        const auto effect = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                                262144 - index, catalogs->key_tables);
        ASSERT_TRUE(effect);
        EXPECT_EQ(*effect, KeyBindingEffect::pane_program_input_changed);
    }

    EXPECT_EQ(input.read_capacity(), empty_capacity - 64);
    EXPECT_TRUE(input.routing_deferred());
    const auto exhausted = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                               262112, catalogs->key_tables);
    ASSERT_TRUE(exhausted);
    EXPECT_EQ(*exhausted, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(input.read_capacity(), empty_capacity - 64);
    input.begin_routing_turn();
    const auto resumed = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                             262112, catalogs->key_tables);
    ASSERT_TRUE(resumed);
    EXPECT_EQ(*resumed, KeyBindingEffect::pane_program_input_changed);
    EXPECT_EQ(input.read_capacity(), empty_capacity - 62);
    EXPECT_FALSE(input.routing_deferred());
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

    const auto first_split = input.route_pending(
        server, ClientId{1}, {session->id, session->link, session->window, session->pane},
        default_prefixes, 262144, catalogs->key_tables);

    ASSERT_TRUE(first_split);
    EXPECT_EQ(*first_split, KeyBindingEffect::client_selection_changed);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2U);
    const auto active_after_first = std::ranges::find(*panes, true, &PaneSnapshot::active);
    ASSERT_NE(active_after_first, panes->end());
    EXPECT_NE(active_after_first->id, session->pane);
    constexpr std::array<char, 2> top_bottom{2, '"'};
    ASSERT_TRUE(input.append({top_bottom.data(), top_bottom.size()}));

    const auto second_split = input.route_pending(
        server, ClientId{1}, {session->id, session->link, session->window, active_after_first->id},
        default_prefixes, 262144, catalogs->key_tables);

    ASSERT_TRUE(second_split);
    EXPECT_EQ(*second_split, KeyBindingEffect::client_selection_changed);
    panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    EXPECT_EQ(panes->size(), 3U);
    EXPECT_EQ(std::ranges::count(*panes, true, &PaneSnapshot::active), 1);
}

/** @brief A large batch of harmless window keys leaves ordered input for another routing turn. */
TEST(ClientKeyInput, LargeKeyBatchYieldsBeforeInputIsExhausted) {
    Server server;
    const auto session = server.create_session("key-burst", "/bin/cat");
    ASSERT_TRUE(session);
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    std::string burst;
    for (unsigned int index = 0; index < 200; ++index) {
        burst.append("\x02"
                     "9");
    }
    ASSERT_TRUE(input.append(burst));
    const auto buffered_capacity = input.read_capacity();

    const auto effect = input.route_pending(
        server, ClientId{1}, {session->id, session->link, session->window, session->pane},
        default_prefixes, 262144, catalogs->key_tables);

    ASSERT_TRUE(effect);
    EXPECT_EQ(*effect, KeyBindingEffect::client_selection_unchanged);
    EXPECT_GT(input.read_capacity(), buffered_capacity);
    EXPECT_LT(input.read_capacity(), empty_capacity);
    EXPECT_TRUE(input.routing_deferred());
    for (unsigned int turn = 0; turn < 16 && input.routing_deferred(); ++turn) {
        input.begin_routing_turn();
        ASSERT_TRUE(input.route_pending(
            server, ClientId{1}, {session->id, session->link, session->window, session->pane},
            default_prefixes, 262144, catalogs->key_tables));
    }
    EXPECT_EQ(input.read_capacity(), empty_capacity);
    EXPECT_FALSE(input.routing_deferred());
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));
}

/** @brief Each navigation binding requires its own registered operation before catalog publication.
 */
TEST(KeyBindingRegistry, WindowKeysRequireRegisteredOperations) {
    for (const bool omit_next : {true, false}) {
        protocol::native::OperationRegistry operations;
        ASSERT_TRUE(operations.add<SelectWindow>());
        if (omit_next) {
            ASSERT_TRUE(operations.add<PreviousWindow>());
        } else {
            ASSERT_TRUE(operations.add<NextWindow>());
        }
        const auto operation_catalog = std::move(operations).freeze();
        ASSERT_TRUE(operation_catalog);
        KeyBindingRegistry registry;
        ASSERT_TRUE(register_window_link_key_bindings(registry));
        const auto rejected = std::move(registry).freeze(*operation_catalog);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
        EXPECT_EQ(rejected.error().operation, omit_next ? "next-window" : "previous-window");
    }
}

/** @brief Sparse window keys wrap within their session and preserve independent shared-window
 * selections. */
TEST(ClientKeyInput, WindowKeysResolveSessionMemberships) {
    Server server;
    const auto first = server.create_session("first", "/bin/cat");
    ASSERT_TRUE(first);
    const auto shared = server.create_window(first->id, 7, "shared", "/bin/cat");
    ASSERT_TRUE(shared);
    const auto second = server.create_session("second", "/bin/cat");
    ASSERT_TRUE(second);
    const auto second_link = server.link_window(shared->window, second->id, 3);
    ASSERT_TRUE(second_link);
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    for (std::uint32_t index = 0; index < 10; ++index) {
        EXPECT_TRUE(catalogs->key_tables.contains("prefix", KeyCode{'0' + index}));
    }

    /** @brief Resolve a session's live selection before routing one complete prefix command. */
    const auto invoke_key = [&](SessionId session_id, char key) -> Result<KeyBindingEffect> {
        const auto sessions = server.sessions();
        const auto session = std::ranges::find(sessions, session_id, &SessionSnapshot::id);
        if (session == sessions.end()) {
            return std::unexpected(TmuxError{TmuxErrorCode::not_found, "test session is absent"});
        }
        ClientKeyInput input;
        const std::array<char, 2> bytes{2, key};
        if (auto queued = input.append({bytes.data(), bytes.size()}); !queued) {
            return std::unexpected(queued.error());
        }
        return input.route_pending(server, ClientId{1},
                                   {session->id, session->link, session->window, session->pane},
                                   default_prefixes, 262144, catalogs->key_tables);
    };

    const auto next = invoke_key(first->id, 'n');
    ASSERT_TRUE(next);
    EXPECT_EQ(*next, KeyBindingEffect::client_selection_changed);
    EXPECT_EQ(server.sessions()[0].link, shared->id);
    EXPECT_EQ(server.sessions()[1].link, second->link);
    ASSERT_TRUE(invoke_key(first->id, 'n'));
    EXPECT_EQ(server.sessions()[0].link, first->link);
    ASSERT_TRUE(invoke_key(first->id, 'p'));
    EXPECT_EQ(server.sessions()[0].link, shared->id);
    ASSERT_TRUE(invoke_key(first->id, 'p'));
    EXPECT_EQ(server.sessions()[0].link, first->link);

    ASSERT_TRUE(invoke_key(second->id, '3'));
    EXPECT_EQ(server.sessions()[1].link, second_link->id);
    EXPECT_EQ(server.sessions()[0].link, first->link);
    ASSERT_TRUE(invoke_key(second->id, '0'));
    EXPECT_EQ(server.sessions()[1].link, second->link);
    const auto unchanged_revision = server.sessions()[1].revision;
    const auto absent = invoke_key(second->id, '9');
    ASSERT_TRUE(absent);
    EXPECT_EQ(*absent, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(server.sessions()[1].revision, unchanged_revision);

    KeyBindingInvocation invocation{server,
                                    ClientId{1},
                                    {first->id, first->link, first->window, first->pane},
                                    default_prefixes.prefix};
    const auto foreign = select_window_link_for_key(invocation, *second_link);
    ASSERT_FALSE(foreign);
    EXPECT_EQ(foreign.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(server.sessions()[0].link, first->link);
    EXPECT_EQ(server.sessions()[1].link, second->link);

    ASSERT_TRUE(server.unlink_window(second_link->id));
    const auto remaining_revision = server.sessions()[1].revision;
    const auto single = invoke_key(second->id, 'n');
    ASSERT_TRUE(single);
    EXPECT_EQ(*single, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(server.sessions()[1].revision, remaining_revision);
    EXPECT_EQ(server.window_links(first->id)->size(), 2U);
}
} // namespace tmux_cxx::key_table
