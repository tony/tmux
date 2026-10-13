#include <tmux_cxx/connection/server_connection.hpp>
#include <tmux_cxx/pane/operations/resize_pane.hpp>
#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/server/server.hpp>

#include "platform/posix/owned_fd.hpp"
#include "protocol/native/framing.hpp"
#include "server/api_catalogs.hpp"
#include "session/session_event_journal.hpp"
#include "session/session_event_reader.hpp"
#include "window/layout.hpp"
#include <tmux_cxx/window/operations/create_window.hpp>
#include <tmux_cxx/window/operations/rename_window.hpp>
#include <tmux_cxx/window/operations/select_layout.hpp>

#include <algorithm>
#include <array>
#include <bit>
#include <fcntl.h>
#include <gtest/gtest.h>
#include <sys/ioctl.h>

namespace tmux_cxx {
/** @brief Reject invalid UTF-8 session names before entity creation or rename effects. */
TEST(ServerSessions, InvalidNameEncodingPreservesState) {
    Server server;
    const std::string invalid_name(1, static_cast<char>(0xff));
    const auto rejected = server.create_session(invalid_name, "/bin/cat");
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_TRUE(server.sessions().empty());
    const auto session = server.create_session("valid", "/bin/cat #" + invalid_name);
    ASSERT_TRUE(session);
    EXPECT_EQ(session->id.value, 1);
    const auto renamed = server.rename_session(session->id, invalid_name);
    ASSERT_FALSE(renamed);
    EXPECT_EQ(renamed.error().code, TmuxErrorCode::invalid_argument);
    ASSERT_EQ(server.sessions().size(), 1);
    EXPECT_EQ(server.sessions().front().name, session->name);
    EXPECT_EQ(server.sessions().front().revision, session->revision);
    const auto window = server.window(session->window);
    ASSERT_TRUE(window);
    EXPECT_EQ(window->name, session->name);
}
/** @brief Session mutations preserve identity and reject stale or duplicate names before
 * allocation. */
TEST(ServerSessions, RenameAndDeletePreserveIdentity) {
    Server server;
    auto created = server.create_session("alpha", "/bin/cat");
    ASSERT_TRUE(created);
    EXPECT_GT(created->pane.value, 0);
    auto duplicate = server.create_session("alpha", "/bin/cat");
    ASSERT_FALSE(duplicate);
    EXPECT_EQ(duplicate.error().code, TmuxErrorCode::duplicate_name);
    ASSERT_TRUE(server.rename_session(created->id, "beta"));
    ASSERT_EQ(server.sessions().size(), 1);
    EXPECT_EQ(server.sessions().front().id, created->id);
    EXPECT_EQ(server.sessions().front().name, "beta");
    ASSERT_TRUE(server.kill_session(created->id));
    auto stale = server.rename_session(created->id, "stale");
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
    EXPECT_TRUE(server.sessions().empty());
}

/** @brief Session mutations append ordered facts while same-name renames remain event-free. */
TEST(SessionEvents, FactsFollowCommittedMutations) {
    Server server;
    const auto event_cursor = SessionEventReader::tail(server);
    const auto created = server.create_session("alpha", "/bin/cat");
    ASSERT_TRUE(created);
    ASSERT_TRUE(server.rename_session(created->id, "alpha"));
    ASSERT_TRUE(server.rename_session(created->id, "beta"));
    ASSERT_TRUE(server.kill_session(created->id));

    const auto session_events = SessionEventReader::read(server, event_cursor);
    ASSERT_TRUE(session_events);
    ASSERT_EQ(session_events->records.size(), 3U);
    const auto *created_event = std::get_if<SessionCreatedEvent>(&session_events->records[0].event);
    const auto *renamed_event = std::get_if<SessionRenamedEvent>(&session_events->records[1].event);
    const auto *closed_event = std::get_if<SessionClosedEvent>(&session_events->records[2].event);
    ASSERT_NE(created_event, nullptr);
    ASSERT_NE(renamed_event, nullptr);
    ASSERT_NE(closed_event, nullptr);
    EXPECT_EQ(created_event->session.id, created->id);
    EXPECT_EQ(created_event->session.window, created->window);
    EXPECT_EQ(created_event->session.name, "alpha");
    EXPECT_EQ(renamed_event->session, created->id);
    EXPECT_EQ(renamed_event->name, "beta");
    EXPECT_GT(renamed_event->revision, created_event->session.revision);
    EXPECT_EQ(closed_event->session, created->id);
    EXPECT_EQ(closed_event->name, "beta");
    EXPECT_GT(closed_event->revision, renamed_event->revision);
    EXPECT_EQ(session_events->records[0].sequence, 0U);
    EXPECT_EQ(session_events->records[1].sequence, 1U);
    EXPECT_EQ(session_events->records[2].sequence, 2U);
    EXPECT_EQ(session_events->next_cursor.next_sequence, 3U);

    Server other;
    const auto wrong_server = SessionEventReader::read(other, session_events->next_cursor);
    ASSERT_FALSE(wrong_server);
    EXPECT_EQ(wrong_server.error().code, TmuxErrorCode::wrong_server);
}

/** @brief A session snapshot and its event cursor share one mutation boundary. */
TEST(SessionEvents, ObservationStartsAfterItsSnapshot) {
    Server server;
    const auto created = server.create_session("alpha", "/bin/cat");
    ASSERT_TRUE(created);

    const auto observation = SessionEventReader::observe_sessions(server);
    ASSERT_EQ(observation.sessions.size(), 1U);
    EXPECT_EQ(observation.sessions.front().name, "alpha");

    ASSERT_TRUE(server.rename_session(created->id, "beta"));
    const auto changes = SessionEventReader::read(server, observation.next_event);
    ASSERT_TRUE(changes);
    ASSERT_EQ(changes->records.size(), 1U);
    const auto *renamed = std::get_if<SessionRenamedEvent>(&changes->records.front().event);
    ASSERT_NE(renamed, nullptr);
    EXPECT_EQ(renamed->session, created->id);
    EXPECT_EQ(renamed->name, "beta");
}

/** @brief Bounded event retention reports an explicit gap instead of fabricating continuity. */
TEST(SessionEvents, RetentionReportsObservationGap) {
    SessionEventJournal journal(2);
    journal.append(SessionClosedEvent{SessionId{1}, "one", 1});
    journal.append(SessionClosedEvent{SessionId{2}, "two", 2});
    journal.append(SessionClosedEvent{SessionId{3}, "three", 3});

    const auto discarded = journal.read_from(0);
    ASSERT_FALSE(discarded);
    EXPECT_EQ(discarded.error().code, TmuxErrorCode::observation_gap);
    const auto retained = journal.read_from(1);
    ASSERT_TRUE(retained);
    ASSERT_EQ(retained->size(), 2U);
    EXPECT_EQ(retained->front().sequence, 1U);
    EXPECT_EQ(journal.tail_sequence(), 3U);
}
/** @brief Invalid creation arguments and malformed native traffic preserve all existing memberships
 * and PTYs. */
TEST(ServerWindows, RejectedCreationPreservesGraphAndPrograms) {
    Server server;
    const auto session = server.create_session("windows", "/bin/cat");
    ASSERT_TRUE(session);
    const auto initial_descriptors = server.pane_pty_descriptors().size();
    struct InvalidCreation {
        std::uint32_t index;
        std::string name;
        std::string command;
        TmuxErrorCode error;
    };
    const std::array cases{
        InvalidCreation{0, "duplicate", "/bin/cat", TmuxErrorCode::duplicate_index},
        InvalidCreation{2147483648U, "overflow", "/bin/cat", TmuxErrorCode::invalid_argument},
        InvalidCreation{1, "", "/bin/cat", TmuxErrorCode::invalid_argument},
        InvalidCreation{1, std::string(129, 'a'), "/bin/cat", TmuxErrorCode::invalid_argument},
        InvalidCreation{1, std::string(1, static_cast<char>(0xff)), "/bin/cat",
                        TmuxErrorCode::invalid_argument},
        InvalidCreation{1, "control\n", "/bin/cat", TmuxErrorCode::invalid_argument},
        InvalidCreation{1, "command", std::string("cat\0ignored", 11),
                        TmuxErrorCode::invalid_argument}};
    for (const auto &invalid : cases) {
        const auto result =
            server.create_window(session->id, invalid.index, invalid.name, invalid.command);
        ASSERT_FALSE(result);
        EXPECT_EQ(result.error().code, invalid.error);
        EXPECT_EQ(server.pane_pty_descriptors().size(), initial_descriptors);
        EXPECT_EQ(server.window_links(session->id)->size(), 1U);
        EXPECT_EQ(server.sessions().front().revision, session->revision);
    }
    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    protocol::native::NativePayloadWriter request;
    request.u64(session->id.value);
    request.u32(1);
    request.string("malformed");
    request.string("/bin/cat");
    request.bytes.push_back(0);
    const auto malformed =
        catalogs->native_operations.invoke(CreateWindow::description.id, server, request.bytes);
    ASSERT_FALSE(malformed);
    EXPECT_EQ(malformed.error().code, TmuxErrorCode::protocol);
    EXPECT_EQ(server.pane_pty_descriptors().size(), initial_descriptors);
    EXPECT_EQ(server.window_links(session->id)->size(), 1U);
    const auto created = server.create_window(session->id, 1, "tools:build.1", "/bin/cat");
    ASSERT_TRUE(created);
    EXPECT_EQ(created->window.value, 2U);
    EXPECT_EQ(server.window(created->window)->pane.value, 2U);
    ASSERT_TRUE(server.unlink_window(created->id));
    const auto stale = server.select_window(created->id);
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
    EXPECT_EQ(server.sessions().front().link, session->link);
}

/** @brief Shared-window names change once, preserve memberships, and reject invalid or stale
 * requests. */
TEST(ServerWindows, RenamePreservesMembershipsAndRejectsInvalidNames) {
    Server server;
    const auto first = server.create_session("first", "/bin/cat");
    const auto second = server.create_session("second", "/bin/cat");
    ASSERT_TRUE(first);
    ASSERT_TRUE(second);
    const auto link = server.link_window(first->window, second->id, 7);
    ASSERT_TRUE(link);
    const auto before = server.window(first->window);
    ASSERT_TRUE(before);
    const auto selected_before = server.sessions();

    const auto renamed = server.rename_window(first->window, "tools:build.1");
    ASSERT_TRUE(renamed);
    EXPECT_EQ(renamed->id, first->window);
    EXPECT_EQ(renamed->pane, first->pane);
    EXPECT_EQ(renamed->name, "tools:build.1");
    EXPECT_GT(renamed->revision, before->revision);
    EXPECT_EQ(server.window(link->window)->name, renamed->name);
    EXPECT_EQ(server.sessions()[0].link, selected_before[0].link);
    EXPECT_EQ(server.sessions()[1].link, selected_before[1].link);
    EXPECT_EQ(server.window_links(second->id)->back().id, link->id);
    const auto repeated = server.rename_window(first->window, renamed->name);
    ASSERT_TRUE(repeated);
    EXPECT_EQ(repeated->revision, renamed->revision);

    const std::array invalid_names{std::string{}, std::string(129, 'x'), std::string("control\n"),
                                   std::string(1, static_cast<char>(0xff)),
                                   std::string("nul\0", 4)};
    for (const auto &invalid : invalid_names) {
        const auto rejected = server.rename_window(first->window, invalid);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::invalid_argument);
        EXPECT_EQ(server.window(first->window)->name, renamed->name);
        EXPECT_EQ(server.window(first->window)->revision, renamed->revision);
    }
    const auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    protocol::native::NativePayloadWriter malformed;
    malformed.u64(first->window.value);
    malformed.string("must-not-rename");
    malformed.bytes.push_back(0);
    const auto rejected =
        catalogs->native_operations.invoke(RenameWindow::description.id, server, malformed.bytes);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::protocol);
    EXPECT_EQ(server.window(first->window)->name, renamed->name);

    ASSERT_TRUE(server.kill_session(first->id));
    ASSERT_TRUE(server.unlink_window(link->id));
    const auto stale = server.rename_window(first->window, "stale");
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
    EXPECT_EQ(server.window(second->window)->name, "second");
}

/** @brief Linked windows keep their panes alive until the final session membership is removed. */
TEST(ServerWindows, MembershipOwnsIndexAndSharesPaneLifetime) {
    Server server;
    auto first = server.create_session("first", "/bin/cat");
    auto second = server.create_session("second", "/bin/cat");
    ASSERT_TRUE(first);
    ASSERT_TRUE(second);
    auto link = server.link_window(first->window, second->id, 7);
    ASSERT_TRUE(link);
    EXPECT_NE(link->id, first->link);
    EXPECT_EQ(link->session, second->id);
    EXPECT_EQ(link->window, first->window);
    EXPECT_EQ(link->index, 7);
    auto duplicate = server.link_window(first->window, second->id, 7);
    ASSERT_FALSE(duplicate);
    EXPECT_EQ(duplicate.error().code, TmuxErrorCode::duplicate_index);
    ASSERT_TRUE(server.kill_session(first->id));
    ASSERT_TRUE(server.send_pane_input(first->pane, "still linked\n"));
    auto links = server.window_links(second->id);
    ASSERT_TRUE(links);
    ASSERT_EQ(links->size(), 2);
    EXPECT_EQ(links->back().id, link->id);
    ASSERT_TRUE(server.unlink_window(second->link));
    EXPECT_EQ(server.sessions().front().window, first->window);
    EXPECT_EQ(server.sessions().front().pane, first->pane);
    auto removed = server.read_pane_output(second->pane);
    ASSERT_FALSE(removed);
    EXPECT_EQ(removed.error().code, TmuxErrorCode::not_found);
    ASSERT_TRUE(server.unlink_window(link->id));
    EXPECT_TRUE(server.sessions().empty());
    EXPECT_FALSE(server.read_pane_output(first->pane));
}

/** @brief A left-right split gives both panes exact nonoverlapping geometry and selects the new
 * pane. */
TEST(ServerPanes, LeftRightSplitOwnsGeometryAndSelection) {
    Server server;
    auto session = server.create_session("left-right", "/bin/cat");
    ASSERT_TRUE(session);
    auto split = server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(split);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2);
    EXPECT_EQ(panes->at(0).id, session->pane);
    EXPECT_EQ(panes->at(0).top, 0);
    EXPECT_EQ(panes->at(0).left, 0);
    EXPECT_EQ(panes->at(0).rows, 24);
    EXPECT_EQ(panes->at(0).columns, 40);
    EXPECT_FALSE(panes->at(0).active);
    EXPECT_EQ(split->id, panes->at(1).id);
    EXPECT_EQ(split->window, session->window);
    EXPECT_EQ(split->top, 0);
    EXPECT_EQ(split->left, 41);
    EXPECT_EQ(split->rows, 24);
    EXPECT_EQ(split->columns, 39);
    EXPECT_TRUE(split->active);
    EXPECT_EQ(split->revision, panes->at(0).revision);
    EXPECT_EQ(server.sessions().front().pane, split->id);
    auto window = server.window(session->window);
    ASSERT_TRUE(window);
    EXPECT_EQ(window->pane, split->id);
    EXPECT_EQ(window->revision, split->revision);
}

/** @brief Pane selection updates one shared window once and rejects foreign identities without
 * effects. */
TEST(ServerPanes, SelectPaneOwnsWindowSelection) {
    Server server;
    auto session = server.create_session("selection", "/bin/cat");
    ASSERT_TRUE(session);
    auto split = server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(split);

    auto selected = server.select_pane(session->pane);
    ASSERT_TRUE(selected);
    EXPECT_EQ(selected->id, session->pane);
    EXPECT_TRUE(selected->active);
    EXPECT_GT(selected->revision, split->revision);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2);
    EXPECT_TRUE(panes->at(0).active);
    EXPECT_FALSE(panes->at(1).active);
    EXPECT_EQ(server.window(session->window)->pane, session->pane);

    const auto selected_revision = selected->revision;
    auto selected_again = server.select_pane(session->pane);
    ASSERT_TRUE(selected_again);
    EXPECT_EQ(selected_again->revision, selected_revision);
    auto missing = server.select_pane(PaneId{99999});
    ASSERT_FALSE(missing);
    EXPECT_EQ(missing.error().code, TmuxErrorCode::not_found);
    EXPECT_EQ(server.window(session->window)->revision, selected_revision);
}

/** @brief A top-bottom split reserves one border row and reports the new lower pane exactly. */
TEST(ServerPanes, TopBottomSplitOwnsGeometry) {
    Server server;
    auto session = server.create_session("top-bottom", "/bin/cat");
    ASSERT_TRUE(session);
    auto split = server.split_pane(session->pane, PaneSplitOrientation::top_bottom, "/bin/cat");
    ASSERT_TRUE(split);
    auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 2);
    EXPECT_EQ(panes->at(0).top, 0);
    EXPECT_EQ(panes->at(0).left, 0);
    EXPECT_EQ(panes->at(0).rows, 12);
    EXPECT_EQ(panes->at(0).columns, 80);
    EXPECT_EQ(split->top, 13);
    EXPECT_EQ(split->left, 0);
    EXPECT_EQ(split->rows, 11);
    EXPECT_EQ(split->columns, 80);
}

/** @brief Named layouts retain linked ownership and logical order while rejecting malformed native
 * traffic. */
TEST(ServerWindows, SelectLayoutPreservesPaneOrderAndMemberships) {
    Server server;
    const auto session = server.create_session("layout", "/bin/cat");
    const auto other = server.create_session("layout-link", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(other);
    const auto right =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    const auto lower =
        server.split_pane(session->pane, PaneSplitOrientation::top_bottom, "/bin/cat");
    ASSERT_TRUE(right);
    ASSERT_TRUE(lower);
    const auto link = server.link_window(session->window, other->id, 7);
    ASSERT_TRUE(link);
    ASSERT_TRUE(server.select_pane(session->pane));
    const auto arranged =
        server.select_layout(session->window, WindowLayoutPreset::main_horizontal_mirrored);
    ASSERT_TRUE(arranged);
    const auto panes = server.panes(session->window);
    ASSERT_TRUE(panes);
    ASSERT_EQ(panes->size(), 3);
    EXPECT_EQ(panes->at(0).id, session->pane);
    EXPECT_EQ(panes->at(1).id, lower->id);
    EXPECT_EQ(panes->at(2).id, right->id);
    EXPECT_EQ(panes->at(0).top, 2);
    EXPECT_EQ(arranged->pane, session->pane);
    EXPECT_EQ(server.window(link->window)->revision, arranged->revision);
    EXPECT_EQ(server.sessions().front().link, session->link);
    EXPECT_EQ(server.sessions().back().link, other->link);
    EXPECT_EQ(server.select_layout(session->window, WindowLayoutPreset::main_horizontal_mirrored)
                  ->revision,
              arranged->revision);

    const auto inserted =
        server.split_pane(lower->id, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(inserted);
    const auto reordered = server.panes(session->window);
    ASSERT_TRUE(reordered);
    ASSERT_EQ(reordered->size(), 4);
    EXPECT_EQ(reordered->at(0).id, session->pane);
    EXPECT_EQ(reordered->at(1).id, lower->id);
    EXPECT_EQ(reordered->at(2).id, inserted->id);
    EXPECT_EQ(reordered->at(3).id, right->id);
    const auto even = server.select_layout(session->window, WindowLayoutPreset::even_horizontal);
    ASSERT_TRUE(even);
    EXPECT_EQ(even->pane, inserted->id);
    for (const auto value : {std::uint32_t{7}, std::uint32_t{65536}}) {
        const auto invalid =
            server.select_layout(session->window, std::bit_cast<WindowLayoutPreset>(value));
        ASSERT_FALSE(invalid);
        EXPECT_EQ(invalid.error().code, TmuxErrorCode::invalid_argument);
    }
    const auto stale = server.select_layout(WindowId{999}, WindowLayoutPreset::tiled);
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);
    ServerConnection unavailable("_build/layout-unreachable");
    const auto narrowed = unavailable.select_layout(
        session->window, std::bit_cast<WindowLayoutPreset>(std::uint32_t{65536}));
    ASSERT_FALSE(narrowed);
    EXPECT_EQ(narrowed.error().code, TmuxErrorCode::invalid_argument);
    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    for (const auto preset : {std::uint16_t{7}, std::uint16_t{1}}) {
        protocol::native::NativePayloadWriter request;
        request.u64(session->window.value);
        request.u16(preset);
        if (preset == 1) {
            request.bytes.push_back(0);
        }
        const auto rejected =
            catalogs->native_operations.invoke(SelectLayout::description.id, server, request.bytes);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::protocol);
        EXPECT_EQ(server.window(session->window)->revision, even->revision);
    }
}

/** @brief Shared-window resizing preserves selection and committed geometry through invalid traffic
 * and terminal failures. */
TEST(ServerPanes, ResizePreservesSharedSelectionAndRejectsInvalidTraffic) {
    Server server;
    const auto session = server.create_session("resize", "/bin/cat");
    const auto other = server.create_session("linked", "/bin/cat");
    ASSERT_TRUE(session);
    ASSERT_TRUE(other);
    const auto split =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(split);
    const auto shared = server.link_window(session->window, other->id, 7);
    ASSERT_TRUE(shared);
    ASSERT_TRUE(server.select_window(shared->id));
    const auto resized = server.resize_pane(session->pane, PaneResizeDirection::right, 5);
    ASSERT_TRUE(resized);
    EXPECT_EQ(resized->columns, 45);
    EXPECT_FALSE(resized->active);
    EXPECT_GT(resized->revision, split->revision);
    EXPECT_EQ(server.window(session->window)->pane, split->id);
    EXPECT_EQ(server.sessions().back().pane, split->id);
    EXPECT_EQ(server.read_pane_text(session->pane)->columns, 45);
    EXPECT_EQ(server.read_pane_text(split->id)->columns, 34);
    EXPECT_EQ(server.resize_pane(session->pane, PaneResizeDirection::right, 0)->revision,
              resized->revision);
    EXPECT_EQ(server.resize_pane(session->pane, PaneResizeDirection::up, 5)->revision,
              resized->revision);
    const auto invalid =
        server.resize_pane(session->pane, std::bit_cast<PaneResizeDirection>(std::uint32_t{4}), 1);
    ASSERT_FALSE(invalid);
    EXPECT_EQ(invalid.error().code, TmuxErrorCode::invalid_argument);
    const auto stale = server.resize_pane(PaneId{999}, PaneResizeDirection::left, 1);
    ASSERT_FALSE(stale);
    EXPECT_EQ(stale.error().code, TmuxErrorCode::not_found);

    ServerConnection connection("_build/pane-resize-unreachable");
    const auto invalid_wire_direction = connection.resize_pane(
        session->pane, std::bit_cast<PaneResizeDirection>(std::uint32_t{65536}), 1);
    ASSERT_FALSE(invalid_wire_direction);
    EXPECT_EQ(invalid_wire_direction.error().code, TmuxErrorCode::invalid_argument);

    auto catalogs = build_server_api_catalogs();
    ASSERT_TRUE(catalogs);
    for (const auto direction : {std::uint16_t{4}, std::uint16_t{1}}) {
        protocol::native::NativePayloadWriter request;
        request.u64(session->pane.value);
        request.u16(direction);
        request.u32(5);
        if (direction == 1) {
            request.bytes.push_back(0);
        }
        const auto rejected =
            catalogs->native_operations.invoke(ResizePane::description.id, server, request.bytes);
        ASSERT_FALSE(rejected);
        EXPECT_EQ(rejected.error().code, TmuxErrorCode::protocol);
        EXPECT_EQ(server.window(session->window)->revision, resized->revision);
        EXPECT_EQ(server.panes(session->window)->front().columns, 45);
    }

    const auto descriptors = server.pane_pty_descriptors();
    const auto target_descriptor =
        std::ranges::find(descriptors, split->id, &std::pair<PaneId, int>::first);
    ASSERT_NE(target_descriptor, descriptors.end());
    posix::OwnedFd saved{::dup(target_descriptor->second)};
    posix::OwnedFd non_terminal{::open("/dev/null", O_RDWR | O_CLOEXEC)};
    ASSERT_GE(saved.get(), 0);
    ASSERT_GE(non_terminal.get(), 0);
    ASSERT_EQ(::dup3(non_terminal.get(), target_descriptor->second, O_CLOEXEC),
              target_descriptor->second);
    const auto failed = server.resize_pane(session->pane, PaneResizeDirection::right, 5);
    EXPECT_EQ(::dup3(saved.get(), target_descriptor->second, O_CLOEXEC), target_descriptor->second);
    ASSERT_FALSE(failed);
    EXPECT_EQ(failed.error().code, TmuxErrorCode::io);
    EXPECT_EQ(server.window(session->window)->revision, resized->revision);
    EXPECT_EQ(server.read_pane_text(session->pane)->columns, 45);
    EXPECT_EQ(server.read_pane_text(split->id)->columns, 34);
    const auto first_descriptor =
        std::ranges::find(descriptors, session->pane, &std::pair<PaneId, int>::first);
    ASSERT_NE(first_descriptor, descriptors.end());
    winsize accepted{};
    ASSERT_EQ(::ioctl(first_descriptor->second, TIOCGWINSZ, &accepted), 0);
    EXPECT_EQ(accepted.ws_col, 45);
}

/** @brief Border movement follows nested ancestors and ordered donors without resizing the window.
 */
TEST(WindowLayout, PaneBordersPreserveNestedMinimumsAndDonorOrder) {
    WindowLayout layout(PaneId{1}, {24, 80});
    ASSERT_TRUE(layout.split(PaneId{1}, PaneId{2}, PaneSplitOrientation::left_right));
    ASSERT_TRUE(layout.split(PaneId{1}, PaneId{3}, PaneSplitOrientation::top_bottom));
    auto changed = layout.resize_pane(PaneId{3}, PaneResizeDirection::right, 5);
    ASSERT_TRUE(changed);
    ASSERT_TRUE(*changed);
    EXPECT_EQ(layout.pane(PaneId{1})->size.columns, 45);
    EXPECT_EQ(layout.pane(PaneId{3})->size.columns, 45);
    EXPECT_EQ(layout.pane(PaneId{2})->left, 46U);
    ASSERT_TRUE(layout.resize_pane(PaneId{2}, PaneResizeDirection::left, 7));
    EXPECT_EQ(layout.pane(PaneId{2})->size.columns, 41);
    ASSERT_TRUE(layout.resize_pane(PaneId{3}, PaneResizeDirection::up, 3));
    EXPECT_EQ(layout.pane(PaneId{1})->size.rows, 9);
    EXPECT_EQ(layout.pane(PaneId{3})->size.rows, 14);
    EXPECT_EQ(layout.pane(PaneId{3})->top, 10U);
    ASSERT_TRUE(layout.resize_pane(PaneId{3}, PaneResizeDirection::right, UINT32_MAX));
    EXPECT_EQ(layout.pane(PaneId{2})->size.columns, 1);
    ASSERT_TRUE(layout.resize_pane(PaneId{3}, PaneResizeDirection::up, UINT32_MAX));
    EXPECT_EQ(layout.pane(PaneId{1})->size.rows, 1);
    EXPECT_EQ(layout.pane(PaneId{3})->size.rows, 22);
    EXPECT_EQ(layout.size(), (terminal::CellSize{24, 80}));
    EXPECT_EQ(layout.resize_pane(PaneId{3}, PaneResizeDirection::up, 1), false);
    EXPECT_EQ(layout.resize_pane(PaneId{3}, PaneResizeDirection::down, 0), false);

    WindowLayout siblings(PaneId{1}, {24, 80});
    ASSERT_TRUE(siblings.split(PaneId{1}, PaneId{2}, PaneSplitOrientation::left_right));
    ASSERT_TRUE(siblings.split(PaneId{2}, PaneId{3}, PaneSplitOrientation::left_right));
    ASSERT_TRUE(siblings.resize_pane(PaneId{2}, PaneResizeDirection::right, UINT32_MAX));
    EXPECT_EQ(siblings.pane(PaneId{1})->size.columns, 1);
    EXPECT_EQ(siblings.pane(PaneId{2})->size.columns, 76);
    EXPECT_EQ(siblings.pane(PaneId{3})->size.columns, 1);
    ASSERT_TRUE(siblings.resize_pane(PaneId{3}, PaneResizeDirection::left, UINT32_MAX));
    EXPECT_EQ(siblings.pane(PaneId{2})->size.columns, 1);
    EXPECT_EQ(siblings.pane(PaneId{3})->size.columns, 76);
    EXPECT_EQ(siblings.resize_pane(PaneId{2}, PaneResizeDirection::up, 7), false);
    EXPECT_FALSE(siblings.resize_pane(PaneId{999}, PaneResizeDirection::right, 1));
    EXPECT_FALSE(
        siblings.resize_pane(PaneId{2}, std::bit_cast<PaneResizeDirection>(std::uint32_t{4}), 1));
}

/** @brief Whole-window shrink retains recursive pane minima and derives every nested offset. */
TEST(WindowLayout, NestedResizePreservesSubtreeMinimums) {
    WindowLayout layout(PaneId{1}, {24, 80});
    ASSERT_TRUE(layout.split(PaneId{1}, PaneId{2}, PaneSplitOrientation::left_right));
    ASSERT_TRUE(layout.split(PaneId{1}, PaneId{3}, PaneSplitOrientation::top_bottom));
    ASSERT_TRUE(layout.split(PaneId{1}, PaneId{4}, PaneSplitOrientation::left_right));

    const auto resized = layout.resize({1, 1});
    ASSERT_TRUE(resized);
    EXPECT_EQ(*resized, (terminal::CellSize{3, 5}));
    const auto panes = layout.panes();
    ASSERT_EQ(panes.size(), 4);
    EXPECT_EQ(panes[0].pane, PaneId{1});
    EXPECT_EQ(panes[0].top, 0U);
    EXPECT_EQ(panes[0].left, 0U);
    EXPECT_EQ(panes[0].size, (terminal::CellSize{1, 1}));
    EXPECT_EQ(panes[1].pane, PaneId{4});
    EXPECT_EQ(panes[1].top, 0U);
    EXPECT_EQ(panes[1].left, 2U);
    EXPECT_EQ(panes[1].size, (terminal::CellSize{1, 1}));
    EXPECT_EQ(panes[2].pane, PaneId{3});
    EXPECT_EQ(panes[2].top, 2U);
    EXPECT_EQ(panes[2].left, 0U);
    EXPECT_EQ(panes[2].size, (terminal::CellSize{1, 3}));
    EXPECT_EQ(panes[3].pane, PaneId{2});
    EXPECT_EQ(panes[3].top, 0U);
    EXPECT_EQ(panes[3].left, 4U);
    EXPECT_EQ(panes[3].size, (terminal::CellSize{3, 1}));
}
} // namespace tmux_cxx
