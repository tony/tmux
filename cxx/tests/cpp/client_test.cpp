#include "client/client_terminal.hpp"
#include "client/client_terminal_routing.hpp"
#include "client/client_view.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "platform/posix/owned_fd.hpp"
#include "server/api_catalogs.hpp"
#include "terminal/terminal_state.hpp"

#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>
#include <tmux_cxx/protocol/tmux/protocol_quirk.hpp>
#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <array>
#include <cerrno>
#include <fcntl.h>
#include <gtest/gtest.h>
#include <poll.h>
#include <pty.h>
#include <sys/ioctl.h>
#include <termios.h>
#include <unistd.h>

namespace tmux_cxx {
namespace {
/** @brief Compare terminal input and local-mode ownership fields restored after attachment. */
bool same_terminal_mode(const termios &left, const termios &right) {
    return left.c_iflag == right.c_iflag && left.c_oflag == right.c_oflag &&
           left.c_cflag == right.c_cflag && left.c_lflag == right.c_lflag;
}
} // namespace

/** @brief Terminal ownership restores when no output byte remains for view teardown. */
TEST(ClientTerminal, RestoreOwnershipAfterViewBackpressure) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    termios original_mode{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &original_mode), 0);
    const auto original_flags = ::fcntl(slave_owner.get(), F_GETFL);
    ASSERT_GE(original_flags, 0);

    auto terminal = ClientTerminal::adopt(posix::OwnedFd(::dup(slave_owner.get())),
                                          posix::OwnedFd(::dup(slave_owner.get())));
    ASSERT_TRUE(terminal);
    ASSERT_TRUE((*terminal)->activate());
    ASSERT_TRUE((*terminal)->queue_output("current view"));

    int pressure_pipe[2]{-1, -1};
    ASSERT_EQ(::pipe2(pressure_pipe, O_CLOEXEC | O_NONBLOCK), 0);
    posix::OwnedFd pressure_reader(pressure_pipe[0]);
    posix::OwnedFd pressure_writer(pressure_pipe[1]);
    const std::string pressure(4096, 'x');
    bool backpressured = false;
    for (std::size_t accepted = 0; accepted < 2 * 1024 * 1024;) {
        const auto written = ::write(pressure_writer.get(), pressure.data(), pressure.size());
        if (written > 0) {
            accepted += static_cast<std::size_t>(written);
        } else if (written < 0 && errno == EINTR) {
            continue;
        } else {
            backpressured = written < 0 && (errno == EAGAIN || errno == EWOULDBLOCK);
            break;
        }
    }
    ASSERT_TRUE(backpressured);
    bool byte_backpressured = false;
    constexpr char pressure_byte = 'x';
    for (std::size_t extra_bytes = 0; extra_bytes < pressure.size(); ++extra_bytes) {
        const auto written = ::write(pressure_writer.get(), &pressure_byte, 1);
        if (written == 1) {
            continue;
        }
        if (written < 0 && errno == EINTR) {
            continue;
        }
        byte_backpressured = written < 0 && (errno == EAGAIN || errno == EWOULDBLOCK);
        break;
    }
    ASSERT_TRUE(byte_backpressured);
    ASSERT_EQ(::dup2(pressure_writer.get(), (*terminal)->output_descriptor()),
              (*terminal)->output_descriptor());

    const auto restoration = (*terminal)->restore();
    ASSERT_FALSE(restoration);
    EXPECT_EQ(restoration.error().code, TmuxErrorCode::io);
    EXPECT_FALSE((*terminal)->active());
    termios restored_mode{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &restored_mode), 0);
    EXPECT_TRUE(same_terminal_mode(restored_mode, original_mode));
    const auto restored_flags = ::fcntl(slave_owner.get(), F_GETFL);
    ASSERT_GE(restored_flags, 0);
    EXPECT_EQ(restored_flags & O_NONBLOCK, original_flags & O_NONBLOCK);
}

/** @brief Client rendering regenerates copied state and tracks source and viewport freshness. */
TEST(ClientView, RedrawUsesCurrentTerminalState) {
    terminal::TerminalState terminal({2, 4});
    ASSERT_TRUE(terminal.ingest_program_output("\x1b[31mA"));
    const auto terminal_snapshot = terminal.snapshot();
    const ClientWindowViewSnapshot window_view{
        WindowId{1},
        1,
        {2, 4},
        {ClientPaneViewSnapshot{{PaneId{1}, 0, 0, {2, 4}}, terminal_snapshot, true}},
    };
    ClientView view;

    auto redraw = view.compose_redraw(window_view, {2, 4});
    ASSERT_TRUE(redraw);
    EXPECT_NE(redraw->find("\x1b[?1049h"), std::string::npos);
    EXPECT_NE(redraw->find('A'), std::string::npos);
    EXPECT_EQ(redraw->find("\x1b[31mA"), std::string::npos);
    view.record_queued_redraw(window_view, {2, 4});
    EXPECT_FALSE(view.needs_redraw(window_view, {2, 4}));
    EXPECT_TRUE(view.needs_redraw(window_view, {1, 4}));

    ASSERT_TRUE(terminal.ingest_program_output("B"));
    auto changed_view = window_view;
    changed_view.panes.front().terminal = terminal.snapshot();
    EXPECT_TRUE(view.needs_redraw(changed_view, {2, 4}));
}

/** @brief Window redraw composes every pane, layout border, and translated active cursor. */
TEST(ClientView, RedrawComposesCompleteWindowLayout) {
    terminal::TerminalState left_terminal({2, 2});
    terminal::TerminalState right_terminal({2, 2});
    ASSERT_TRUE(left_terminal.ingest_program_output("L"));
    ASSERT_TRUE(right_terminal.ingest_program_output("R"));
    const ClientWindowViewSnapshot window_view{
        WindowId{7},
        11,
        {2, 5},
        {
            ClientPaneViewSnapshot{{PaneId{1}, 0, 0, {2, 2}}, left_terminal.snapshot(), false},
            ClientPaneViewSnapshot{{PaneId{2}, 0, 3, {2, 2}}, right_terminal.snapshot(), true},
        },
    };
    ClientView view;

    const auto redraw = view.compose_redraw(window_view, {2, 5});
    ASSERT_TRUE(redraw);
    EXPECT_NE(redraw->find('L'), std::string::npos);
    EXPECT_NE(redraw->find('R'), std::string::npos);
    EXPECT_NE(redraw->find("\x1b[1;3H\x1b[0m|"), std::string::npos);
    EXPECT_NE(redraw->find("\x1b[1;5H\x1b[?25h"), std::string::npos);

    view.record_queued_redraw(window_view, {2, 5});
    EXPECT_FALSE(view.needs_redraw(window_view, {2, 5}));
    auto changed_layout = window_view;
    ++changed_layout.window_revision;
    EXPECT_TRUE(view.needs_redraw(changed_layout, {2, 5}));
    auto changed_pane = window_view;
    ++changed_pane.panes.front().terminal.revision;
    EXPECT_TRUE(view.needs_redraw(changed_pane, {2, 5}));
}

/** @brief Disconnecting one attached client restores its TTY without destroying retained work. */
TEST(ClientEntities, AttachmentOwnsOnlyClientRelationship) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    const winsize initial_size{10, 40, 0, 0};
    ASSERT_EQ(::ioctl(slave_owner.get(), TIOCSWINSZ, &initial_size), 0);
    termios original{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &original), 0);

    Server server;
    auto session = server.create_session("retained", "/bin/cat");
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
    const auto reported_compatibility = server.stock_client_compatibility(*client);
    ASSERT_TRUE(reported_compatibility);
    EXPECT_EQ(reported_compatibility->mode, protocol::tmux::ConnectionMode::command);
    EXPECT_EQ(reported_compatibility->verification,
              protocol::tmux::CompatibilityVerification::tested_subset);

    auto attached = server.attach_client(*client, session->id);
    ASSERT_TRUE(attached);
    ASSERT_EQ(attached->session, session->id);
    EXPECT_TRUE(attached->identified);
    EXPECT_TRUE(attached->has_terminal);
    EXPECT_GT(attached->rows, 0U);
    EXPECT_GT(attached->columns, 0U);
    auto window = server.window(session->window);
    ASSERT_TRUE(window);
    EXPECT_EQ(window->rows, 10U);
    EXPECT_EQ(window->columns, 40U);
    const auto clients = server.clients();
    ASSERT_EQ(clients.size(), 1U);
    EXPECT_EQ(clients.front().id, *client);
    termios raw{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &raw), 0);
    EXPECT_EQ(raw.c_lflag & (ICANON | ECHO | ISIG), 0U);

    const winsize resized_terminal{12, 42, 0, 0};
    ASSERT_EQ(::ioctl(slave_owner.get(), TIOCSWINSZ, &resized_terminal), 0);
    auto resized = server.resize_client(*client);
    ASSERT_TRUE(resized);
    EXPECT_EQ(resized->rows, 12U);
    EXPECT_EQ(resized->columns, 42U);
    window = server.window(session->window);
    ASSERT_TRUE(window);
    EXPECT_EQ(window->rows, 12U);
    EXPECT_EQ(window->columns, 42U);

    ASSERT_EQ(ClientTerminalRouting::capture(server).size(), 1U);
    auto detached = server.detach_client(*client);
    ASSERT_TRUE(detached);
    EXPECT_FALSE(detached->session);
    EXPECT_EQ(detached->detached_from, "retained");
    auto api_catalogs = build_server_api_catalogs();
    ASSERT_TRUE(api_catalogs);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, 0, 0),
        ClientTerminalRouteOutcome::superseded);
    termios detached_mode{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &detached_mode), 0);
    EXPECT_TRUE(same_terminal_mode(detached_mode, original));

    ASSERT_TRUE(server.attach_client(*client, session->id));
    ASSERT_TRUE(server.kill_session(session->id));
    const auto killed_session_clients = server.clients();
    ASSERT_EQ(killed_session_clients.size(), 1U);
    EXPECT_FALSE(killed_session_clients.front().session);
    EXPECT_EQ(killed_session_clients.front().detached_from, "retained");
    EXPECT_TRUE(server.sessions().empty());

    auto survivor = server.create_session("survivor", "/bin/cat");
    ASSERT_TRUE(survivor);
    ASSERT_TRUE(server.attach_client(*client, survivor->id));
    StockClientLifecycle::disconnect(server, *client);
    termios restored{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &restored), 0);
    EXPECT_TRUE(same_terminal_mode(restored, original));
    ASSERT_EQ(server.sessions().size(), 1U);
    EXPECT_EQ(server.sessions().front().id, survivor->id);
}

/** @brief Root input, send-prefix, and detach preserve split-read key-table behavior. */
TEST(ClientKeyInput, RootAndPrefixTablesRouteAcrossReadBoundaries) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    termios original{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &original), 0);

    Server server;
    const auto session = server.create_session("prefix", "/bin/cat");
    ASSERT_TRUE(session);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    auto compatibility = protocol::tmux::TmuxCompatibilityPolicy{}.assess(
        protocol::tmux::legacy_profile,
        protocol::tmux::ProtocolDirection::stock_client_to_cxx_server,
        protocol::tmux::ConnectionMode::interactive);
    ASSERT_TRUE(StockClientLifecycle::identify_terminal(
        server, *client, posix::OwnedFd(::dup(slave_owner.get())),
        posix::OwnedFd(::dup(slave_owner.get())), std::move(compatibility)));
    ASSERT_TRUE(server.attach_client(*client, session->id));
    auto api_catalogs = build_server_api_catalogs();
    ASSERT_TRUE(api_catalogs);

    constexpr char root_input = 'z';
    ASSERT_EQ(::write(master_owner.get(), &root_input, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    EXPECT_TRUE(server.pane_pty_write_pending(session->pane));
    server.advance_pane_pty_io(session->pane);
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));

    constexpr char prefix = '\x02';
    ASSERT_EQ(::write(master_owner.get(), &prefix, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));
    ASSERT_EQ(server.clients().front().session, session->id);

    constexpr char unknown = 'x';
    ASSERT_EQ(::write(master_owner.get(), &unknown, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));

    ASSERT_EQ(::write(master_owner.get(), &prefix, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));
    ASSERT_EQ(::write(master_owner.get(), &prefix, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    EXPECT_TRUE(server.pane_pty_write_pending(session->pane));
    server.advance_pane_pty_io(session->pane);
    EXPECT_FALSE(server.pane_pty_write_pending(session->pane));

    constexpr char detach = 'd';
    ASSERT_EQ(::write(master_owner.get(), &prefix, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    ASSERT_EQ(::write(master_owner.get(), &detach, 1), 1);
    EXPECT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);
    ASSERT_EQ(server.sessions().size(), 1U);
    ASSERT_EQ(server.clients().size(), 1U);
    EXPECT_FALSE(server.clients().front().session);
    EXPECT_EQ(server.clients().front().detached_from, "prefix");
    termios restored{};
    ASSERT_EQ(::tcgetattr(slave_owner.get(), &restored), 0);
    EXPECT_TRUE(same_terminal_mode(restored, original));
}

/** @brief The default prefix-o binding selects the next pane through the typed pane operation. */
TEST(ClientKeyInput, SelectNextPaneBindingResolvesItsTargetAtExecution) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);

    Server server;
    const auto session = server.create_session("key-panes", "/bin/cat");
    ASSERT_TRUE(session);
    const auto second =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(second);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    auto compatibility = protocol::tmux::TmuxCompatibilityPolicy{}.assess(
        protocol::tmux::legacy_profile,
        protocol::tmux::ProtocolDirection::stock_client_to_cxx_server,
        protocol::tmux::ConnectionMode::interactive);
    ASSERT_TRUE(StockClientLifecycle::identify_terminal(
        server, *client, posix::OwnedFd(::dup(slave_owner.get())),
        posix::OwnedFd(::dup(slave_owner.get())), std::move(compatibility)));
    ASSERT_TRUE(server.attach_client(*client, session->id));
    auto api_catalogs = build_server_api_catalogs();
    ASSERT_TRUE(api_catalogs);

    constexpr std::array<char, 3> input{'\x02', 'o', 'z'};
    ASSERT_EQ(::write(master_owner.get(), input.data(), input.size()),
              static_cast<ssize_t>(input.size()));
    ASSERT_EQ(
        ClientTerminalRouting::route_ready_io(server, *client, api_catalogs->key_tables, POLLIN, 0),
        ClientTerminalRouteOutcome::routed);

    const auto selected_window = server.window(session->window);
    ASSERT_TRUE(selected_window);
    EXPECT_EQ(selected_window->pane, session->pane);
    EXPECT_FALSE(server.pane_pty_write_pending(second->id));
    EXPECT_TRUE(server.pane_pty_write_pending(session->pane));
}

/** @brief A control client attaches without terminal ownership or window-size arbitration. */
TEST(ClientEntities, ControlAttachmentOwnsOnlySessionRelationship) {
    Server server;
    const auto session = server.create_session("control", "/bin/cat");
    ASSERT_TRUE(session);
    const auto original_window = server.window(session->window);
    ASSERT_TRUE(original_window);
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    auto compatibility = protocol::tmux::TmuxCompatibilityPolicy{}.assess(
        protocol::tmux::legacy_profile,
        protocol::tmux::ProtocolDirection::stock_client_to_cxx_server,
        protocol::tmux::ConnectionMode::control);
    ASSERT_TRUE(StockClientLifecycle::identify_command(server, *client, std::move(compatibility)));

    const auto attached = server.attach_client(*client, session->id);
    ASSERT_TRUE(attached);
    EXPECT_EQ(attached->session, session->id);
    EXPECT_TRUE(attached->identified);
    EXPECT_FALSE(attached->has_terminal);
    EXPECT_EQ(attached->rows, 0U);
    EXPECT_EQ(attached->columns, 0U);
    const auto attached_window = server.window(session->window);
    ASSERT_TRUE(attached_window);
    EXPECT_EQ(attached_window->rows, original_window->rows);
    EXPECT_EQ(attached_window->columns, original_window->columns);

    const auto detached = server.detach_client(*client);
    ASSERT_TRUE(detached);
    EXPECT_FALSE(detached->session);
    EXPECT_EQ(detached->detached_from, "control");
}

/** @brief Client-size arbitration resizes the complete window layout and every pane terminal. */
TEST(ClientEntities, AttachmentResizesEveryPaneInSelectedWindow) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    const winsize client_size{10, 40, 0, 0};
    ASSERT_EQ(::ioctl(slave_owner.get(), TIOCSWINSZ, &client_size), 0);

    Server server;
    const auto session = server.create_session("split-resize", "/bin/cat");
    ASSERT_TRUE(session);
    const auto split =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(split);
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

    const auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2);
    EXPECT_EQ(panes->at(0).rows, 10U);
    EXPECT_EQ(panes->at(0).columns, 20U);
    EXPECT_EQ(panes->at(1).rows, 10U);
    EXPECT_EQ(panes->at(1).columns, 19U);
    EXPECT_EQ(panes->at(1).left, 21U);
    const auto first_text = server.read_pane_text(panes->at(0).id);
    const auto second_text = server.read_pane_text(panes->at(1).id);
    ASSERT_TRUE(first_text);
    ASSERT_TRUE(second_text);
    EXPECT_EQ(first_text->rows, 10U);
    EXPECT_EQ(first_text->columns, 20U);
    EXPECT_EQ(second_text->rows, 10U);
    EXPECT_EQ(second_text->columns, 19U);
    StockClientLifecycle::disconnect(server, *client);
}

/** @brief Zero TTY dimensions activate the declared 24x80 compatibility adjustment. */
TEST(ClientEntities, ZeroTerminalDimensionsActivateNamedQuirk) {
    int master = -1;
    int slave = -1;
    ASSERT_EQ(::openpty(&master, &slave, nullptr, nullptr, nullptr), 0);
    posix::OwnedFd master_owner(master);
    posix::OwnedFd slave_owner(slave);
    const winsize zero_size{};
    ASSERT_EQ(::ioctl(slave_owner.get(), TIOCSWINSZ, &zero_size), 0);

    Server server;
    const auto client = StockClientLifecycle::accept(server);
    ASSERT_TRUE(client);
    auto compatibility = protocol::tmux::TmuxCompatibilityPolicy{}.assess(
        protocol::tmux::legacy_profile,
        protocol::tmux::ProtocolDirection::stock_client_to_cxx_server,
        protocol::tmux::ConnectionMode::command);
    ASSERT_TRUE(StockClientLifecycle::identify_terminal(
        server, *client, posix::OwnedFd(::dup(slave_owner.get())),
        posix::OwnedFd(::dup(slave_owner.get())), std::move(compatibility)));

    const auto clients = server.clients();
    ASSERT_EQ(clients.size(), 1U);
    EXPECT_EQ(clients.front().rows, 24U);
    EXPECT_EQ(clients.front().columns, 80U);
    const auto report = server.stock_client_compatibility(*client);
    ASSERT_TRUE(report);
    const auto name = protocol::tmux::protocol_quirk(
                          protocol::tmux::ProtocolQuirk::zero_terminal_dimensions_use_defaults)
                          .name;
    EXPECT_TRUE(std::ranges::contains(report->quirks, name));
    EXPECT_FALSE(report->limitations.empty());
    StockClientLifecycle::disconnect(server, *client);
}
} // namespace tmux_cxx
