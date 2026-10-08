#include "pane/program.hpp"

#include <gtest/gtest.h>
#include <sys/wait.h>
#include <unistd.h>

namespace tmux_cxx {
/** @brief Unexpected external child collection relinquishes ownership without inventing an exit
 * status or retaining a signal target. */
TEST(PaneProgram, ExternalReapingRelinquishesOwnership) {
    PaneProgram program;
    const auto child = ::fork();
    ASSERT_GE(child, 0);
    if (child == 0) {
        ::_exit(0);
    }
    ASSERT_TRUE(program.adopt(child));
    int status = 0;
    ASSERT_EQ(::waitpid(child, &status, 0), child);
    program.stop();
    EXPECT_TRUE(program.reaped());
    EXPECT_FALSE(program.wait_status().has_value());
    EXPECT_EQ(program.readiness_descriptor(), -1);
}
} // namespace tmux_cxx
