#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/server/server.hpp>

#include "session/session_event_journal.hpp"
#include "session/session_event_reader.hpp"
#include "window/layout.hpp"

#include <gtest/gtest.h>

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
