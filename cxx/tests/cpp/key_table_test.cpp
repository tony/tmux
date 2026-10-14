#include "client/client_key_input.hpp"
#include "client/client_terminal.hpp"
#include "client/client_terminal_routing.hpp"
#include "client/operations/detach_client_key_binding.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "key_table/key_binding_registry.hpp"
#include "pane/operations/copy_mode_key_bindings.hpp"
#include "pane/operations/select_next_pane_key_binding.hpp"
#include "pane/pane.hpp"
#include "protocol/native/operation_registry.hpp"
#include "server/api_catalogs.hpp"
#include "window_link/operations/select_window_key_binding.hpp"
#include "window_link/register_key_bindings.hpp"
#include <tmux_cxx/window_link/operations/next_window.hpp>
#include <tmux_cxx/window_link/operations/previous_window.hpp>
#include <tmux_cxx/window_link/operations/select_window.hpp>

#include <tmux_cxx/client/operations/detach_client.hpp>
#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>
#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <array>
#include <chrono>
#include <fcntl.h>
#include <gtest/gtest.h>
#include <poll.h>
#include <pty.h>
#include <termios.h>
#include <unistd.h>
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
int failed_binding_calls = 0;    ///< Count permanent failures without hiding a repeated failed key.

/** @brief Return a permanent operation error while leaving the pane's frozen mode unchanged. */
struct FailingCopyBinding {
    static constexpr KeyBindingDescription description{
        "copy-mode", control_key('w'), "copy-selection", DetachClient::description.id};
    ///< Isolate interactive error recovery behind validated operation-backed registration.
    /** @brief Reject before effects so recovery must consume the failed key and retain mode. */
    static Result<KeyBindingEffect> execute(KeyBindingInvocation &) {
        ++failed_binding_calls;
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "selection exceeds the buffer limit"});
    }
};

/** @brief Advance only the owned pane through readiness until its retained output contains text. */
bool await_pane_output(Server &server, PaneId pane, std::string_view text) {
    using Clock = std::chrono::steady_clock;
    const auto deadline = Clock::now() + std::chrono::milliseconds(500);
    const auto descriptors = server.pane_pty_descriptors();
    const auto descriptor = std::ranges::find(descriptors, pane, &std::pair<PaneId, int>::first);
    if (descriptor == descriptors.end()) {
        return false;
    }
    while (Clock::now() < deadline) {
        server.advance_pane_pty_io(pane);
        const auto output = server.read_pane_output(pane);
        if (output && output->find(text) != std::string::npos) {
            return true;
        }
        const auto remaining =
            std::chrono::ceil<std::chrono::milliseconds>(deadline - Clock::now());
        pollfd readiness{descriptor->second, POLLIN, 0};
        if (::poll(&readiness, 1, static_cast<int>(std::max<std::int64_t>(0, remaining.count()))) <=
            0) {
            return false;
        }
    }
    return false;
}

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

/** @brief A permanent matched failure consumes only its key and preserves mode plus retained
 * suffix. */
TEST(ClientKeyInput, PermanentActionFailureConsumesOnlyFailedKey) {
    Server server;
    const auto session = server.create_session("failed-copy-key", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(server.enter_copy_mode(session->pane));
    const auto before = server.copy_mode(session->pane);
    ASSERT_TRUE(before);
    ASSERT_TRUE(before->mode);
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<FailingCopyBinding>());
    auto key_tables = std::move(registry).freeze(*operation_catalog);
    ASSERT_TRUE(key_tables);
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    ASSERT_TRUE(input.append("\x17\x1b[B"));
    failed_binding_calls = 0;
    const ClientSelection selection{session->id, session->link, session->window, session->pane};

    const auto failed = input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                            *key_tables, "copy-mode");

    ASSERT_FALSE(failed);
    EXPECT_EQ(failed.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(failed.error().operation, "copy-selection");
    EXPECT_EQ(input.read_capacity(), empty_capacity - 3);
    EXPECT_EQ(failed_binding_calls, 1);
    const auto retained = server.copy_mode(session->pane);
    ASSERT_TRUE(retained);
    ASSERT_TRUE(retained->mode);
    EXPECT_EQ(retained->display_revision, before->display_revision);
    EXPECT_EQ(retained->mode->lines, before->mode->lines);
    const auto suffix = input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                            *key_tables, "copy-mode");
    EXPECT_TRUE(suffix);
    EXPECT_EQ(failed_binding_calls, 1);
    EXPECT_EQ(input.read_capacity(), empty_capacity);
}

/** @brief A rejected matched action reports its error while preserving the attachment and frozen
 * mode. */
TEST(ClientTerminalRouting, PermanentActionFailureKeepsAttachmentAndReportsError) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    ASSERT_EQ(::fcntl(master_owner.get(), F_SETFL, O_NONBLOCK), 0);
    Server server;
    const auto session = server.create_session("reported-copy-error", "/bin/cat");
    ASSERT_TRUE(session);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    auto compatibility = protocol::tmux::TmuxCompatibilityPolicy{}.assess(
        protocol::tmux::legacy_profile,
        protocol::tmux::ProtocolDirection::stock_client_to_cxx_server,
        protocol::tmux::ConnectionMode::command);
    ASSERT_TRUE(StockClientLifecycle::identify_terminal(
        server, *client, posix::OwnedFd(::dup(slave_owner.get())),
        posix::OwnedFd(::dup(slave_owner.get())), std::move(compatibility)));
    ASSERT_TRUE(server.attach_client(*client, session->id));
    ASSERT_TRUE(
        server.set_window_option(WindowOptionTarget{session->window}, "mode-keys", "emacs"));
    ASSERT_TRUE(server.enter_copy_mode(session->pane));
    const auto before = server.copy_mode(session->pane);
    ASSERT_TRUE(before);
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<FailingCopyBinding>());
    ASSERT_TRUE((registry.add<CancelCopyModeKeyBinding<WindowModeKeys::emacs, escape_key>>()));
    auto key_tables = std::move(registry).freeze(catalogs->native_operations);
    ASSERT_TRUE(key_tables);
    failed_binding_calls = 0;
    ASSERT_EQ(::write(master_owner.get(), "\x17\x1b[B", 4), 4);

    const auto failure =
        ClientTerminalRouting::route_ready_io(server, *client, *key_tables, POLLIN, 0);

    EXPECT_EQ(failure, ClientTerminalRouteOutcome::action_failed);
    EXPECT_EQ(failed_binding_calls, 1);
    ASSERT_EQ(server.clients().size(), 1U);
    EXPECT_EQ(server.clients().front().session, session->id);
    const auto retained = server.copy_mode(session->pane);
    ASSERT_TRUE(retained);
    ASSERT_TRUE(retained->mode);
    EXPECT_EQ(retained->display_revision, before->display_revision);
    EXPECT_EQ(retained->mode->lines, before->mode->lines);
    auto routes = ClientTerminalRouting::capture(server);
    ASSERT_EQ(routes.size(), 1U);
    EXPECT_TRUE(routes.front().output_pending);
    EXPECT_FALSE(routes.front().key_deadline);
    EXPECT_FALSE(routes.front().key_routing_deferred);
    EXPECT_EQ(ClientTerminalRouting::route_ready_io(server, *client, *key_tables, 0, POLLOUT),
              ClientTerminalRouteOutcome::routed);
    EXPECT_EQ(failed_binding_calls, 1);
    std::array<char, 512> message{};
    const auto received = ::read(master_owner.get(), message.data(), message.size());
    ASSERT_GT(received, 0);
    EXPECT_NE(std::string_view(message.data(), static_cast<std::size_t>(received))
                  .find("selection exceeds the buffer limit"),
              std::string_view::npos);
    EXPECT_EQ(server.clients().front().session, session->id);

    using namespace std::chrono_literals;
    const auto start = TerminalKeyDecoder::Clock::time_point{};
    ASSERT_EQ(::write(master_owner.get(), "\x1b", 1), 1);
    EXPECT_EQ(ClientTerminalRouting::route_ready_io(server, *client, *key_tables, POLLIN, 0, start),
              ClientTerminalRouteOutcome::routed);
    routes = ClientTerminalRouting::capture(server);
    ASSERT_EQ(routes.size(), 1U);
    EXPECT_EQ(routes.front().key_deadline, start + 10ms);
    EXPECT_FALSE(routes.front().key_routing_deferred);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, *key_tables, 0, 0, start + 10ms),
        ClientTerminalRouteOutcome::routed);
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    EXPECT_EQ(server.clients().front().session, session->id);
    EXPECT_FALSE(ClientTerminalRouting::capture(server).front().key_deadline);
}

/** @brief Full view output retains a bounded latest error and delivers it without truncating the
 * view. */
TEST(ClientTerminal, LatestErrorSurvivesFullOutputQueue) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    ASSERT_EQ(::fcntl(master_owner.get(), F_SETFL, O_NONBLOCK), 0);
    auto terminal = ClientTerminal::adopt(posix::OwnedFd(::dup(slave_owner.get())),
                                          posix::OwnedFd(::dup(slave_owner.get())));
    ASSERT_TRUE(terminal);
    ASSERT_TRUE((*terminal)->activate());
    const std::string view(1048576, 'v');
    ASSERT_TRUE((*terminal)->queue_output(view));
    (*terminal)->queue_error({TmuxErrorCode::invalid_argument, "superseded error"});
    (*terminal)->queue_error(
        {TmuxErrorCode::invalid_argument, "latest\x1b[2J error\x7f\n", "copy-selection"});
    std::size_t zero_budget = 0;
    ASSERT_TRUE((*terminal)->flush_output(zero_budget));
    EXPECT_TRUE((*terminal)->output_pending());
    EXPECT_TRUE((*terminal)->active());
    std::string delivered;
    delivered.reserve(view.size() + 512);
    std::array<char, 8192> bytes{};
    for (unsigned turn = 0; turn < 512 && (*terminal)->output_pending(); ++turn) {
        std::size_t budget = 4096;
        ASSERT_TRUE((*terminal)->flush_output(budget));
        for (;;) {
            const auto received = ::read(master_owner.get(), bytes.data(), bytes.size());
            if (received > 0) {
                delivered.append(bytes.data(), static_cast<std::size_t>(received));
            } else if (received < 0 && errno == EINTR) {
                continue;
            } else {
                ASSERT_EQ(received, -1);
                ASSERT_TRUE(errno == EAGAIN || errno == EWOULDBLOCK);
                break;
            }
        }
    }
    EXPECT_FALSE((*terminal)->output_pending());
    EXPECT_TRUE((*terminal)->active());
    ASSERT_GE(delivered.size(), view.size());
    EXPECT_EQ(std::string_view(delivered).substr(0, view.size()), view);
    const auto error = std::string_view(delivered).substr(view.size());
    EXPECT_NE(error.find("copy-selection: latest[2J error"), std::string_view::npos);
    EXPECT_EQ(error.find("superseded error"), std::string_view::npos);
    EXPECT_EQ(error.find("latest\x1b[2J"), std::string_view::npos);
    EXPECT_EQ(error.find('\x7f'), std::string_view::npos);
    EXPECT_EQ(error.find('\n'), std::string_view::npos);
    EXPECT_LE(error.size(), 512U);
}

/** @brief Fragmented raw keys retain complete encoded spans under backpressure and reach the
 * program unchanged. */
TEST(ClientKeyInput, FragmentedPassthroughPreservesAtomicRawKeys) {
    using namespace std::chrono_literals;
    const auto start = TerminalKeyDecoder::Clock::time_point{};
    Server server;
    const auto session =
        server.create_session("raw-key-spans", "stty raw -echo; printf ready; exec cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(await_pane_output(server, session->pane, "ready"));
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    const ClientSelection selection{session->id, session->link, session->window, session->pane};
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    ASSERT_TRUE(input.append("\xe4"));
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 2,
                                    catalogs->key_tables, "root", start));
    EXPECT_EQ(input.next_key_deadline(), start + 10ms);
    ASSERT_TRUE(input.append("\xb8\xad\x1b["));
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 2,
                                    catalogs->key_tables, "root", start + 1ms));
    EXPECT_EQ(input.read_capacity(), empty_capacity - 5);
    EXPECT_FALSE(input.next_key_deadline());
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));
    ASSERT_TRUE(input.append("A\n"));
    const auto unicode = input.route_pending(server, ClientId{1}, selection, default_prefixes, 3,
                                             catalogs->key_tables, "root", start + 2ms);
    ASSERT_TRUE(unicode);
    EXPECT_EQ(*unicode, KeyBindingEffect::pane_program_input_changed);
    EXPECT_EQ(input.read_capacity(), empty_capacity - 4);
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 2,
                                    catalogs->key_tables, "root", start + 3ms));
    EXPECT_EQ(input.read_capacity(), empty_capacity - 4);
    EXPECT_FALSE(input.next_key_deadline());
    const auto arrow = input.route_pending(server, ClientId{1}, selection, default_prefixes, 3,
                                           catalogs->key_tables, "root", start + 4ms);
    ASSERT_TRUE(arrow);
    EXPECT_EQ(*arrow, KeyBindingEffect::pane_program_input_changed);
    EXPECT_EQ(input.read_capacity(), empty_capacity - 1);
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 1,
                                    catalogs->key_tables, "root", start + 5ms));
    EXPECT_EQ(input.read_capacity(), empty_capacity);
    constexpr std::string_view raw{"\xe4\xb8\xad\x1b[A\n"};
    ASSERT_TRUE(await_pane_output(server, session->pane, raw));
    EXPECT_EQ(*server.read_pane_output(session->pane), std::string("ready") + std::string(raw));
}

/** @brief Persistent copy tables consume fragmented arrows and literal Escape expires without
 * another read. */
TEST(ClientKeyInput, CopyTableKeepsArrowsAndEscapeDeadline) {
    using namespace std::chrono_literals;
    const auto start = TerminalKeyDecoder::Clock::time_point{};
    Server server;
    const auto session = server.create_session("copy-key-spans", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(server.enter_copy_mode(session->pane));
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    const ClientSelection selection{session->id, session->link, session->window, session->pane};
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    ASSERT_TRUE(input.append("\x1b["));
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                    catalogs->key_tables, "copy-mode", start));
    EXPECT_EQ(input.next_key_deadline(), start + 10ms);
    ASSERT_TRUE(input.append("Bx\x1b[B"));
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                    catalogs->key_tables, "copy-mode", start + 1ms));
    EXPECT_EQ(server.copy_mode(session->pane)->mode->cursor.row, 2U);
    EXPECT_EQ(input.read_capacity(), empty_capacity);
    EXPECT_FALSE(input.next_key_deadline());
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));
    ASSERT_TRUE(input.append("\x1b"));
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                    catalogs->key_tables, "copy-mode", start + 2ms));
    EXPECT_EQ(input.next_key_deadline(), start + 12ms);
    const auto cancelled = input.route_pending(server, ClientId{1}, selection, default_prefixes, 0,
                                               catalogs->key_tables, "copy-mode", start + 12ms);
    ASSERT_TRUE(cancelled);
    EXPECT_EQ(*cancelled, KeyBindingEffect::pane_mode_changed);
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    EXPECT_FALSE(input.next_key_deadline());
    EXPECT_EQ(input.read_capacity(), empty_capacity);
}

/** @brief Program EOF releases blocked passthrough while preserving later prefix controls and their
 * suffix. */
TEST(ClientKeyInput, ProgramEofReleasesPassthroughBeforePrefix) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    Pane pane(PaneId{1}, WindowId{1}, {24, 80});
    pane.pty.reset(master);
    posix::OwnedFd slave_owner(slave);
    ASSERT_EQ(::fcntl(pane.pty.get(), F_SETFL, O_NONBLOCK), 0);
    termios mode{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &mode), 0);
    mode.c_lflag &= ~ECHO;
    ASSERT_EQ(::tcsetattr(slave_owner.get(), TCSANOW, &mode), 0);
    ASSERT_TRUE(pane.admit_program_input(std::string(262144, 'a')));

    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    KeyBindingRegistry registry;
    ASSERT_TRUE(registry.add<RootFallbackBinding>());
    const auto key_tables = std::move(registry).freeze(*operation_catalog);
    ASSERT_TRUE(key_tables);
    Server server;
    ClientKeyInput input;
    const auto empty_capacity = input.read_capacity();
    const ClientSelection selection{SessionId{1}, WindowLinkId{1}, WindowId{1}, PaneId{1}};
    ASSERT_TRUE(input.append("drop\x02xkeep"));
    const auto queued_capacity = input.read_capacity();
    const auto backpressured =
        input.route_pending(server, ClientId{1}, selection, default_prefixes,
                            pane.client_program_input_capacity(), *key_tables);
    ASSERT_TRUE(backpressured);
    EXPECT_EQ(*backpressured, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(input.read_capacity(), queued_capacity);
    EXPECT_FALSE(input.routing_deferred());

    ASSERT_EQ(::write(slave_owner.get(), "retained", 8), 8);
    slave_owner.reset();
    pane.advance_pty_io();
    ASSERT_EQ(pane.pty.get(), -1);
    ASSERT_FALSE(pane.program_input.accepting_input());
    ASSERT_EQ(pane.output.retained_bytes(), "retained");

    const auto control = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                             pane.client_program_input_capacity(), *key_tables);
    ASSERT_TRUE(control);
    EXPECT_EQ(*control, KeyBindingEffect::client_selection_changed);
    EXPECT_EQ(input.read_capacity(), empty_capacity - 4);
    const auto suffix = input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                            pane.client_program_input_capacity(), *key_tables);
    ASSERT_TRUE(suffix);
    EXPECT_EQ(*suffix, KeyBindingEffect::client_selection_unchanged);
    EXPECT_EQ(input.read_capacity(), empty_capacity);

    std::string burst;
    for (unsigned int index = 0; index < 64; ++index) {
        burst.append("drop\x02!");
    }
    ASSERT_TRUE(input.append(burst));
    const auto buffered_capacity = input.read_capacity();
    input.begin_routing_turn();
    ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                    pane.client_program_input_capacity(), *key_tables));
    EXPECT_TRUE(input.routing_deferred());
    // 64 decisions consume 21 complete groups and the next passthrough.
    EXPECT_EQ(input.read_capacity(), buffered_capacity + 21 * 6 + 4);
    for (unsigned int turn = 0; turn < 3 && input.routing_deferred(); ++turn) {
        input.begin_routing_turn();
        ASSERT_TRUE(input.route_pending(server, ClientId{1}, selection, default_prefixes,
                                        pane.client_program_input_capacity(), *key_tables));
    }
    EXPECT_EQ(input.read_capacity(), empty_capacity);
    EXPECT_FALSE(input.routing_deferred());
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
