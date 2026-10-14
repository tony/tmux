#include "pane/copy_mode.hpp"
#include "terminal/terminal_state.hpp"

#include <gtest/gtest.h>

#include <bit>

#include <limits>

namespace tmux_cxx {
namespace {
/** @brief Collect displayed glyphs without inventing text for wide-cell padding. */
std::u32string screen_text(const PaneScreenSnapshot &screen) {
    std::u32string text;
    for (const auto &cell : screen.cells) {
        text += cell.text;
    }
    return text;
}
} // namespace

/** @brief Frozen history supports bounded navigation and reverse selection independently of later
 * parser output. */
TEST(CopyMode, FrozenBackingNavigationAndSelection) {
    terminal::TerminalState terminal({2, 4}, 4);
    ASSERT_TRUE(terminal.ingest_program_output("abcdEF\r\nGH\r\nIJ"));
    const auto source = terminal.history_snapshot();
    auto mode = CopyMode::capture(source, WindowModeKeys::emacs);
    ASSERT_TRUE(mode);
    EXPECT_EQ(mode->state().cursor, (CopyPosition{3, 2}));
    EXPECT_EQ(mode->state().viewport_top, 2U);
    EXPECT_EQ(mode->state().retained_rows, 4U);
    EXPECT_EQ(mode->state().source_history_lines, 2U);
    EXPECT_EQ(mode->state().source_revision, source.revision);
    ASSERT_TRUE(mode->move_to(CopyCursorDestination::history_start, WindowModeKeys::emacs));
    EXPECT_TRUE(mode->begin_selection());
    ASSERT_TRUE(mode->move_to(CopyCursorDestination::history_end, WindowModeKeys::emacs));
    EXPECT_EQ(mode->selected_text(WindowModeKeys::emacs, 256), "abcdEF\nGH\nIJ");
    EXPECT_TRUE(mode->clear_selection(WindowModeKeys::emacs));
    EXPECT_TRUE(mode->begin_selection());
    ASSERT_TRUE(mode->move_to(CopyCursorDestination::history_start, WindowModeKeys::emacs));
    EXPECT_EQ(mode->selected_text(WindowModeKeys::emacs, 256), "abcdEF\nGH\nIJ");
    const auto before_failure = mode->state();
    const auto oversized = mode->selected_text(WindowModeKeys::emacs, 4);
    ASSERT_FALSE(oversized);
    EXPECT_EQ(oversized.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(mode->state(), before_failure);
    EXPECT_FALSE(mode->move(std::bit_cast<CopyCursorDirection>(std::uint32_t{99}), 1,
                            WindowModeKeys::emacs));
    EXPECT_FALSE(mode->move(CopyCursorDirection::right, 1,
                            std::bit_cast<WindowModeKeys>(std::uint32_t{99})));
    EXPECT_EQ(mode->state(), before_failure);
    const auto clamped = mode->move(
        CopyCursorDirection::up, std::numeric_limits<std::uint32_t>::max(), WindowModeKeys::emacs);
    ASSERT_TRUE(clamped);
    EXPECT_FALSE(*clamped);
    const auto display = mode->screen_snapshot();
    ASSERT_TRUE(terminal.ingest_program_output("\r\nnew parser rows"));
    EXPECT_EQ(mode->screen_snapshot().cells, display.cells);
    EXPECT_EQ(mode->state().source_revision, source.revision);

    terminal::TerminalState scrolling({5, 8}, 8);
    ASSERT_TRUE(scrolling.ingest_program_output(
        "r0\r\nr1\r\nr2\r\nr3\r\nr4\r\nr5\r\nr6\r\nr7\r\nr8\r\nr9"));
    auto paged = CopyMode::capture(scrolling.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(paged);
    ASSERT_TRUE(paged->scroll(CopyScrollMotion::page_up, 1, WindowModeKeys::emacs));
    EXPECT_EQ(paged->state().viewport_top, 2U);
    EXPECT_EQ(paged->state().cursor.row, 6U);
    EXPECT_EQ(paged->state().cursor.column, 2U);
    ASSERT_TRUE(paged->scroll(CopyScrollMotion::page_down, 1, WindowModeKeys::emacs));
    EXPECT_EQ(paged->state().viewport_top, 5U);
    EXPECT_EQ(paged->state().cursor.row, 9U);
    ASSERT_TRUE(paged->scroll(CopyScrollMotion::line_up, 2, WindowModeKeys::emacs));
    EXPECT_EQ(paged->state().viewport_top, 3U);
    EXPECT_EQ(paged->state().cursor.row, 7U);
}

/** @brief Counted vertical motion preserves per-row vi affinity and source-defined wide-cell
 * endpoint columns. */
TEST(CopyMode, CountedVerticalMotionPreservesIntermediateCursorAffinity) {
    terminal::TerminalState terminal({3, 12});
    ASSERT_TRUE(terminal.ingest_program_output("abcdefghij\r\nxyz\r\n1234567890"));
    auto counted = CopyMode::capture(terminal.history_snapshot(), WindowModeKeys::vi);
    ASSERT_TRUE(counted);
    ASSERT_TRUE(counted->move_to(CopyCursorDestination::line_start, WindowModeKeys::vi));
    ASSERT_TRUE(counted->move(CopyCursorDirection::right, 4, WindowModeKeys::vi));
    auto repeated = *counted;
    ASSERT_TRUE(counted->move(CopyCursorDirection::up, 2, WindowModeKeys::vi));
    ASSERT_TRUE(repeated.move(CopyCursorDirection::up, 1, WindowModeKeys::vi));
    ASSERT_TRUE(repeated.move(CopyCursorDirection::up, 1, WindowModeKeys::vi));
    EXPECT_EQ(counted->state().cursor, (CopyPosition{0, 2}));
    EXPECT_EQ(counted->state().cursor, repeated.state().cursor);

    terminal::TerminalState wide({2, 8});
    ASSERT_TRUE(wide.ingest_program_output("A\u754cB\r\n1234"));
    auto padding = CopyMode::capture(wide.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(padding);
    ASSERT_TRUE(padding->move(CopyCursorDirection::left, 2, WindowModeKeys::emacs));
    ASSERT_TRUE(padding->move(CopyCursorDirection::up, 1, WindowModeKeys::emacs));
    EXPECT_EQ(padding->state().cursor, (CopyPosition{0, 2}));

    terminal::TerminalState endpoint({1, 4});
    ASSERT_TRUE(endpoint.ingest_program_output("abc"));
    const auto entered_vi = CopyMode::capture(endpoint.history_snapshot(), WindowModeKeys::vi);
    ASSERT_TRUE(entered_vi);
    EXPECT_EQ(entered_vi->state().cursor, (CopyPosition{0, 3}));
    auto live_keys = CopyMode::capture(endpoint.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(live_keys);
    EXPECT_TRUE(live_keys->begin_selection());
    const auto before_invalid = live_keys->state();
    EXPECT_FALSE(live_keys->clear_selection(std::bit_cast<WindowModeKeys>(std::uint32_t{99})));
    EXPECT_EQ(live_keys->state(), before_invalid);
    const auto cleared = live_keys->clear_selection(WindowModeKeys::vi);
    ASSERT_TRUE(cleared);
    EXPECT_TRUE(*cleared);
    EXPECT_FALSE(live_keys->state().selection_anchor);
    EXPECT_EQ(live_keys->state().cursor, (CopyPosition{0, 2}));
    const auto unchanged = live_keys->clear_selection(WindowModeKeys::vi);
    ASSERT_TRUE(unchanged);
    EXPECT_FALSE(*unchanged);
}

/** @brief Extraction preserves whole Unicode cells, key-style endpoints, empty rows, and discarded
 * history evidence. */
TEST(CopyMode, SelectionEndpointsPreserveWholeUnicodeCells) {
    terminal::TerminalState terminal({2, 8});
    ASSERT_TRUE(terminal.ingest_program_output("A\u754ce\u0301B"));
    auto mode = CopyMode::capture(terminal.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(mode);
    ASSERT_TRUE(mode->move_to(CopyCursorDestination::history_start, WindowModeKeys::emacs));
    EXPECT_TRUE(mode->begin_selection());
    ASSERT_TRUE(mode->move(CopyCursorDirection::right, 2, WindowModeKeys::emacs));
    EXPECT_EQ(mode->state().cursor, (CopyPosition{0, 3}));
    EXPECT_EQ(mode->selected_text(WindowModeKeys::emacs, 256), "A\u754c");
    EXPECT_EQ(mode->selected_text(WindowModeKeys::vi, 256), "A\u754ce\u0301");
    EXPECT_EQ(mode->state().highlight_keys, WindowModeKeys::emacs);
    EXPECT_TRUE(mode->clear_selection(WindowModeKeys::emacs));
    EXPECT_TRUE(mode->begin_selection());
    ASSERT_TRUE(mode->move(CopyCursorDirection::left, 2, WindowModeKeys::emacs));
    EXPECT_EQ(mode->selected_text(WindowModeKeys::emacs, 256), "A\u754c");
    EXPECT_EQ(mode->selected_text(WindowModeKeys::vi, 256), "A\u754ce\u0301");

    terminal::TerminalState empty({1, 4});
    auto blank = CopyMode::capture(empty.history_snapshot(), WindowModeKeys::vi);
    ASSERT_TRUE(blank);
    EXPECT_TRUE(blank->begin_selection());
    EXPECT_EQ(blank->selected_text(WindowModeKeys::emacs, 256), "");
    EXPECT_EQ(blank->selected_text(WindowModeKeys::vi, 256), "\n");
    terminal::TerminalState evicted({2, 4}, 1);
    ASSERT_TRUE(evicted.ingest_program_output("abcdEF\r\nGH\r\nIJ"));
    auto gap = CopyMode::capture(evicted.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(gap);
    EXPECT_TRUE(gap->state().history_gap);
    ASSERT_TRUE(gap->move_to(CopyCursorDestination::history_start, WindowModeKeys::emacs));
    EXPECT_TRUE(gap->begin_selection());
    ASSERT_TRUE(gap->move_to(CopyCursorDestination::history_end, WindowModeKeys::emacs));
    EXPECT_EQ(gap->selected_text(WindowModeKeys::emacs, 256), "EF\nGH\nIJ");
    ASSERT_TRUE(evicted.ingest_program_output("\x1b[?1049hUV"));
    const auto alternate = CopyMode::capture(evicted.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(alternate);
    EXPECT_TRUE(alternate->state().alternate_screen);
    EXPECT_FALSE(alternate->state().history_gap);
    EXPECT_EQ(alternate->state().source_history_lines, 0U);
}

/** @brief Independent resize candidates preserve occupied spaces and glyphs while clearing
 * selection and remapping the cursor. */
TEST(CopyMode, ResizeCandidatesReflowWithoutChangingOriginal) {
    terminal::TerminalState terminal({2, 4});
    ASSERT_TRUE(terminal.ingest_program_output("ab  CD"));
    auto original = CopyMode::capture(terminal.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(original);
    EXPECT_TRUE(original->begin_selection());
    const auto before = original->state();
    const auto rejected = original->resized({0, 3}, WindowModeKeys::emacs);
    ASSERT_FALSE(rejected);
    EXPECT_EQ(original->state(), before);
    auto smaller = original->resized({2, 3}, WindowModeKeys::emacs);
    ASSERT_TRUE(smaller);
    EXPECT_EQ(original->state(), before);
    EXPECT_FALSE(smaller->state().selection_anchor);
    EXPECT_EQ(smaller->state().cursor, (CopyPosition{1, 3}));
    EXPECT_GT(smaller->state().revision, before.revision);
    EXPECT_EQ(screen_text(smaller->screen_snapshot()), U"ab  CD");
    auto larger = smaller->resized({2, 6}, WindowModeKeys::emacs);
    ASSERT_TRUE(larger);
    EXPECT_EQ(larger->state().cursor, (CopyPosition{0, 6}));
    EXPECT_EQ(screen_text(larger->screen_snapshot()), U"ab  CD      ");
    EXPECT_EQ(larger->state().source_revision, before.source_revision);
    EXPECT_EQ(terminal.snapshot().size, (terminal::CellSize{2, 4}));

    terminal::TerminalState wide({2, 4});
    ASSERT_TRUE(wide.ingest_program_output("A\u754cB"));
    auto glyphs = CopyMode::capture(wide.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(glyphs);
    auto narrow = glyphs->resized({2, 3}, WindowModeKeys::emacs);
    ASSERT_TRUE(narrow);
    ASSERT_TRUE(narrow->move_to(CopyCursorDestination::history_start, WindowModeKeys::emacs));
    EXPECT_TRUE(narrow->begin_selection());
    ASSERT_TRUE(narrow->move_to(CopyCursorDestination::line_end, WindowModeKeys::emacs));
    EXPECT_EQ(narrow->selected_text(WindowModeKeys::emacs, 256), "A\u754cB");

    terminal::TerminalState unfit({2, 4});
    ASSERT_TRUE(unfit.ingest_program_output("\u754c"));
    auto retained = CopyMode::capture(unfit.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(retained);
    const auto retained_before = retained->state();
    EXPECT_FALSE(retained->resized({2, 1}, std::bit_cast<WindowModeKeys>(std::uint32_t{99})));
    EXPECT_EQ(retained->state(), retained_before);
    auto single_column = retained->resized({2, 1}, WindowModeKeys::emacs);
    ASSERT_TRUE(single_column);
    EXPECT_EQ(single_column->state().cursor, (CopyPosition{0, 1}));
    EXPECT_EQ(screen_text(single_column->screen_snapshot()), U"  ");
    auto restored = single_column->resized({2, 4}, WindowModeKeys::emacs);
    ASSERT_TRUE(restored);
    EXPECT_EQ(screen_text(restored->screen_snapshot()), U"\u754c      ");
    auto vi_column = retained->resized({2, 1}, WindowModeKeys::vi);
    ASSERT_TRUE(vi_column);
    EXPECT_EQ(vi_column->state().cursor, (CopyPosition{0, 0}));

    terminal::TerminalState history_width({2, 8}, 4);
    ASSERT_TRUE(history_width.ingest_program_output("abcdefghIJ\r\nKL\r\nMN"));
    ASSERT_TRUE(history_width.resize({2, 4}));
    const auto mixed_width = history_width.history_snapshot();
    ASSERT_EQ(mixed_width.history_lines, 2U);
    ASSERT_EQ(mixed_width.lines.front().used_columns, 8U);
    auto historical = CopyMode::capture(mixed_width, WindowModeKeys::emacs);
    ASSERT_TRUE(historical);
    auto taller = historical->resized({3, 4}, WindowModeKeys::emacs);
    ASSERT_TRUE(taller);
    EXPECT_EQ(taller->state().retained_rows, historical->state().retained_rows);
    EXPECT_EQ(taller->state().source_history_lines, 2U);
}
} // namespace tmux_cxx
