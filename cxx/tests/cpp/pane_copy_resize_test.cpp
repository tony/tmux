#include "pane/pending_copy_resize.hpp"
#include "platform/posix/owned_fd.hpp"
#include "window/window_size.hpp"

#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <array>
#include <cstdlib>
#include <fcntl.h>
#include <gtest/gtest.h>
#include <poll.h>
#include <sys/resource.h>
#include <type_traits>
#include <unistd.h>

namespace tmux_cxx {
static_assert(std::is_nothrow_move_constructible_v<PendingPaneCopyResize>);
namespace {
/** @brief Restore the test process's descriptor limit after forcing a real spawn failure. */
class DescriptorLimitGuard {
  public:
    /** @brief Retain the original descriptor limit before any test mutation. */
    DescriptorLimitGuard() : valid_(::getrlimit(RLIMIT_NOFILE, &original_) == 0) {}
    /** @brief Restore the original limit even when a test exits through an assertion. */
    ~DescriptorLimitGuard() { (void)restore(); }
    /** @brief Prevent new descriptors while leaving existing pane PTYs usable. */
    bool block_allocation() const {
        const rlimit blocked{0, original_.rlim_max};
        return valid_ && ::setrlimit(RLIMIT_NOFILE, &blocked) == 0;
    }
    /** @brief Restore descriptor allocation before later test setup or assertions. */
    bool restore() {
        if (!valid_) {
            return true;
        }
        if (::setrlimit(RLIMIT_NOFILE, &original_) != 0) {
            return false;
        }
        valid_ = false;
        return true;
    }

  private:
    rlimit original_{}; ///< Owned process limit restored at scope exit.
    bool valid_;        ///< Original limit was obtained and still needs restoration.
};

/** @brief Write real output through an owned pane's slave before freezing its terminal state. */
void enter_selected_mode(Server &server, PaneId pane) {
    const auto descriptors = server.pane_pty_descriptors();
    const auto descriptor = std::ranges::find(descriptors, pane, &std::pair<PaneId, int>::first);
    ASSERT_NE(descriptor, descriptors.end());
    std::array<char, 128> slave_name{};
    ASSERT_EQ(::ptsname_r(descriptor->second, slave_name.data(), slave_name.size()), 0);
    posix::OwnedFd slave{::open(slave_name.data(), O_WRONLY | O_NOCTTY | O_CLOEXEC)};
    ASSERT_GE(slave.get(), 0);
    constexpr std::string_view output{"ab  CDEF\r\nGH"};
    ASSERT_EQ(::write(slave.get(), output.data(), output.size()),
              static_cast<ssize_t>(output.size()));
    server.advance_pane_pty_io(pane);
    ASSERT_TRUE(server.read_pane_text(pane)->lines.front().starts_with("ab  CDEF"));
    ASSERT_TRUE(server.enter_copy_mode(pane));
    ASSERT_TRUE(server.move_copy_cursor_to(pane, CopyCursorDestination::history_start));
    ASSERT_TRUE(server.begin_copy_selection(pane));
    ASSERT_TRUE(server.move_copy_cursor(pane, CopyCursorDirection::right, 2));
    ASSERT_TRUE(server.copy_mode(pane)->mode->selection);
}

/** @brief Verify frozen rows, selection, coordinates, and display freshness survived a rejection.
 */
void expect_mode_unchanged(const CopyModeSnapshot &before, const CopyModeSnapshot &after) {
    ASSERT_TRUE(before.mode);
    ASSERT_TRUE(after.mode);
    EXPECT_EQ(after.display_revision, before.display_revision);
    EXPECT_EQ(after.mode->lines, before.mode->lines);
    EXPECT_EQ(after.mode->rows, before.mode->rows);
    EXPECT_EQ(after.mode->columns, before.mode->columns);
    EXPECT_EQ(after.mode->backing_lines, before.mode->backing_lines);
    EXPECT_EQ(after.mode->cursor, before.mode->cursor);
    EXPECT_EQ(after.mode->viewport_top, before.mode->viewport_top);
    EXPECT_EQ(after.mode->source_revision, before.mode->source_revision);
    ASSERT_EQ(after.mode->selection.has_value(), before.mode->selection.has_value());
    if (before.mode->selection) {
        EXPECT_EQ(after.mode->selection->anchor, before.mode->selection->anchor);
        EXPECT_EQ(after.mode->selection->end, before.mode->selection->end);
    }
}
} // namespace

/** @brief A later PTY resize failure preserves all frozen modes; success commits their prepared
 * dimensions once. */
TEST(PaneCopyResize, LaterPaneFailurePreservesRowsSelectionAndEpochs) {
    Server server;
    const auto session = server.create_session("copy-resize", "/bin/cat");
    ASSERT_TRUE(session);
    const auto second =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(second);
    enter_selected_mode(server, session->pane);
    enter_selected_mode(server, second->id);
    const auto first_before = server.copy_mode(session->pane);
    const auto second_before = server.copy_mode(second->id);
    ASSERT_TRUE(first_before);
    ASSERT_TRUE(second_before);
    ASSERT_TRUE(first_before->mode);
    ASSERT_TRUE(second_before->mode);
    ASSERT_TRUE(first_before->mode->selection);
    ASSERT_TRUE(second_before->mode->selection);
    const auto window_before = server.window(session->window);
    ASSERT_TRUE(window_before);
    const auto descriptors = server.pane_pty_descriptors();
    const auto poisoned =
        std::ranges::find(descriptors, second->id, &std::pair<PaneId, int>::first);
    ASSERT_NE(poisoned, descriptors.end());
    posix::OwnedFd saved{::dup(poisoned->second)};
    posix::OwnedFd non_terminal{::open("/dev/null", O_RDWR | O_CLOEXEC)};
    ASSERT_GE(saved.get(), 0);
    ASSERT_GE(non_terminal.get(), 0);
    ASSERT_EQ(::dup3(non_terminal.get(), poisoned->second, O_CLOEXEC), poisoned->second);
    const auto rejected = server.resize_pane(session->pane, PaneResizeDirection::right, 5);
    EXPECT_EQ(::dup3(saved.get(), poisoned->second, O_CLOEXEC), poisoned->second);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::io);
    expect_mode_unchanged(*first_before, *server.copy_mode(session->pane));
    expect_mode_unchanged(*second_before, *server.copy_mode(second->id));
    EXPECT_EQ(server.window(session->window)->revision, window_before->revision);
    EXPECT_EQ(server.read_pane_text(session->pane)->columns, first_before->mode->columns);

    const auto resized = server.resize_pane(session->pane, PaneResizeDirection::right, 5);
    ASSERT_TRUE(resized);
    for (const auto pane : {session->pane, second->id}) {
        const auto committed = server.copy_mode(pane);
        ASSERT_TRUE(committed);
        ASSERT_TRUE(committed->mode);
        EXPECT_EQ(committed->mode->columns, server.read_pane_text(pane)->columns);
        EXPECT_FALSE(committed->mode->selection);
        EXPECT_TRUE(committed->mode->lines.front().starts_with("ab  CDEF"));
        EXPECT_GT(committed->display_revision, pane == session->pane
                                                   ? first_before->display_revision
                                                   : second_before->display_revision);
    }
    const auto after = server.copy_mode(session->pane);
    ASSERT_TRUE(after);
    ASSERT_TRUE(server.resize_pane(session->pane, PaneResizeDirection::right, 0));
    expect_mode_unchanged(*after, *server.copy_mode(session->pane));
}

/** @brief A real split spawn failure discards copy reflow; a successful split commits it afterward.
 */
TEST(PaneCopyResize, SplitSpawnFailurePreservesFrozenMode) {
    Server server;
    const auto session = server.create_session("copy-split", "/bin/cat");
    ASSERT_TRUE(session);
    enter_selected_mode(server, session->pane);
    const auto before = server.copy_mode(session->pane);
    ASSERT_TRUE(before);
    ASSERT_TRUE(before->mode);
    ASSERT_TRUE(before->mode->selection);
    const auto window_before = server.window(session->window);
    ASSERT_TRUE(window_before);
    DescriptorLimitGuard limit;
    ASSERT_TRUE(limit.block_allocation());
    const auto rejected =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(limit.restore());
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::io);
    expect_mode_unchanged(*before, *server.copy_mode(session->pane));
    EXPECT_EQ(server.window(session->window)->revision, window_before->revision);
    EXPECT_EQ(server.panes(session->window)->size(), 1U);
    EXPECT_EQ(server.read_pane_text(session->pane)->columns, before->mode->columns);
    const auto accepted =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(accepted);
    const auto committed = server.copy_mode(session->pane);
    ASSERT_TRUE(committed);
    ASSERT_TRUE(committed->mode);
    EXPECT_EQ(committed->mode->columns, server.read_pane_text(session->pane)->columns);
    EXPECT_FALSE(committed->mode->selection);
    EXPECT_GT(committed->display_revision, before->display_revision);
}

/** @brief Named layouts and client sizing commit frozen display geometry with their shared window.
 */
TEST(PaneCopyResize, LayoutAndClientSizingCommitPreparedModes) {
    Server server;
    const auto session = server.create_session("copy-layout", "/bin/cat");
    ASSERT_TRUE(session);
    const auto second =
        server.split_pane(session->pane, PaneSplitOrientation::left_right, "/bin/cat");
    ASSERT_TRUE(second);
    enter_selected_mode(server, session->pane);
    enter_selected_mode(server, second->id);
    ASSERT_TRUE(server.select_layout(session->window, WindowLayoutPreset::even_vertical));
    for (const auto pane : {session->pane, second->id}) {
        const auto mode = server.copy_mode(pane);
        ASSERT_TRUE(mode);
        ASSERT_TRUE(mode->mode);
        EXPECT_EQ(mode->mode->rows, server.read_pane_text(pane)->rows);
        EXPECT_EQ(mode->mode->columns, server.read_pane_text(pane)->columns);
        EXPECT_FALSE(mode->mode->selection);
        ASSERT_TRUE(server.begin_copy_selection(pane));
    }
    ASSERT_TRUE(LatestClientWindowSize::apply(server, session->id, {10, 60}));
    for (const auto pane : {session->pane, second->id}) {
        const auto mode = server.copy_mode(pane);
        ASSERT_TRUE(mode);
        ASSERT_TRUE(mode->mode);
        EXPECT_EQ(mode->mode->rows, server.read_pane_text(pane)->rows);
        EXPECT_EQ(mode->mode->columns, 60U);
        EXPECT_FALSE(mode->mode->selection);
    }
}

/** @brief Real program EOF preserves pane backing for copy entry, client sizing, and cancellation.
 */
TEST(PaneCopyResize, ProgramEofPreservesCopyEntryResizeAndCancel) {
    Server server;
    const auto session = server.create_session("retained-copy", "printf 'retained screen\\n'");
    ASSERT_TRUE(session);
    const auto exits = server.pane_program_readiness_descriptors();
    ASSERT_EQ(exits.size(), 1U);
    pollfd exit_readiness{exits.front(), POLLIN, 0};
    ASSERT_EQ(::poll(&exit_readiness, 1, 500), 1);
    ASSERT_NE(exit_readiness.revents & POLLIN, 0);
    server.reap_pane_programs();
    EXPECT_TRUE(server.pane_program_readiness_descriptors().empty());
    server.advance_pane_pty_io(session->pane);
    const auto retained = server.read_pane_text(session->pane);
    ASSERT_TRUE(retained);
    EXPECT_FALSE(retained->input_open);
    EXPECT_TRUE(server.pane_pty_descriptors().empty());
    ASSERT_TRUE(retained->lines.front().starts_with("retained screen"));
    const auto closed_input = server.send_pane_input(session->pane, "late");
    ASSERT_FALSE(closed_input);
    EXPECT_EQ(closed_input.error().code, TmuxErrorCode::closed);

    ASSERT_TRUE(server.enter_copy_mode(session->pane));
    const auto entered = server.copy_mode(session->pane);
    ASSERT_TRUE(entered);
    ASSERT_TRUE(entered->mode);
    EXPECT_EQ(entered->mode->source_revision, retained->revision);
    ASSERT_TRUE(LatestClientWindowSize::apply(server, session->id, {12, 40}));
    const auto resized = server.copy_mode(session->pane);
    ASSERT_TRUE(resized);
    ASSERT_TRUE(resized->mode);
    EXPECT_EQ(resized->mode->rows, 12U);
    EXPECT_EQ(resized->mode->columns, 40U);
    EXPECT_EQ(resized->mode->source_revision, entered->mode->source_revision);
    EXPECT_GT(resized->display_revision, entered->display_revision);
    // Copy resize retains all backing rows and puts this history cursor at the viewport top.
    EXPECT_EQ(resized->mode->backing_lines, 24U);
    EXPECT_EQ(resized->mode->cursor.row, 1U);
    EXPECT_EQ(resized->mode->viewport_top, 1U);
    ASSERT_TRUE(server.move_copy_cursor_to(session->pane, CopyCursorDestination::history_start));
    EXPECT_TRUE(
        server.copy_mode(session->pane)->mode->lines.front().starts_with("retained screen"));
    ASSERT_TRUE(server.cancel_copy_mode(session->pane));
    EXPECT_FALSE(server.copy_mode(session->pane)->mode);
    EXPECT_FALSE(server.read_pane_text(session->pane)->input_open);
    EXPECT_TRUE(server.read_pane_text(session->pane)->lines.front().starts_with("retained screen"));
    ASSERT_EQ(server.panes(session->window)->size(), 1U);
    EXPECT_EQ(server.panes(session->window)->front().id, session->pane);
}
} // namespace tmux_cxx
