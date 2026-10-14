#include "pane/pane.hpp"

#include <gtest/gtest.h>

namespace tmux_cxx {
/** @brief Frozen pane views keep freshness stable while parsing advances and refresh on
 * cancellation. */
TEST(PaneCopyMode, DisplayFreshnessSurvivesProgramAndModeCounterCollisions) {
    Pane pane{PaneId{1}, WindowId{1}, {2, 8}};
    ASSERT_TRUE(pane.terminal.ingest_program_output("abc"));
    const auto program = pane.screen_snapshot();
    auto mode = CopyMode::capture(pane.terminal.history_snapshot(), WindowModeKeys::emacs);
    ASSERT_TRUE(mode);
    pane.copy_mode = std::move(*mode);
    pane.touch_copy_display();
    const auto frozen = pane.screen_snapshot();
    EXPECT_GT(frozen.revision, program.revision);
    ASSERT_TRUE(pane.terminal.ingest_program_output("DEF"));
    const auto still_frozen = pane.screen_snapshot();
    EXPECT_EQ(still_frozen.cells, frozen.cells);
    EXPECT_EQ(still_frozen.revision, frozen.revision);
    ASSERT_TRUE(pane.copy_mode->begin_selection());
    pane.touch_copy_display();
    const auto selected = pane.screen_snapshot();
    EXPECT_GT(selected.revision, frozen.revision);
    pane.copy_mode.reset();
    pane.touch_copy_display();
    const auto restored = pane.screen_snapshot();
    EXPECT_GT(restored.revision, selected.revision);
    EXPECT_NE(restored.cells, frozen.cells);
    EXPECT_EQ(restored.revision, pane.display_revision());
}
} // namespace tmux_cxx
