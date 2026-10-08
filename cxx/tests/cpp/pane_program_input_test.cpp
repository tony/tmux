#include "pane/pane_program_input.hpp"

#include <gtest/gtest.h>

namespace tmux_cxx {
/** @brief Queue rejection preserves admitted bytes and a failed descriptor closes input without
 * scheduling more writes. */
TEST(PaneProgramInput, CapacityAndFailurePreserveAdmission) {
    PaneProgramInput input;
    ASSERT_TRUE(input.accepting_input());
    ASSERT_TRUE(input.admit(std::string(262144, 'x')));
    const auto full = input.admit("y");
    ASSERT_FALSE(full);
    EXPECT_EQ(full.error().code, TmuxErrorCode::backpressure);
    EXPECT_EQ(input.remaining_capacity(), 0);
    std::size_t remaining_bytes = 65536;
    const auto failed = input.flush_to_pty(-1, remaining_bytes);
    ASSERT_FALSE(failed);
    EXPECT_EQ(failed.error().code, TmuxErrorCode::io);
    EXPECT_FALSE(input.accepting_input());
    EXPECT_FALSE(input.delivery_pending());
    const auto closed = input.admit("z");
    ASSERT_FALSE(closed);
    EXPECT_EQ(closed.error().code, TmuxErrorCode::closed);
}
} // namespace tmux_cxx
