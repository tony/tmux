#include "protocol/tmux/control/command_line.hpp"

#include <gtest/gtest.h>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Control input preserves quoted arguments and rejects unsupported command groups. */
TEST(ControlCommandLine, ParsesOneRegisteredCommandWithoutGroups) {
    auto parsed = parse_command_line(R"(rename-session -t 'old name' "new name" escaped\ value)");
    ASSERT_TRUE(parsed);
    EXPECT_EQ(*parsed, (std::vector<std::string>{"rename-session", "-t", "old name", "new name",
                                                 "escaped value"}));

    EXPECT_FALSE(parse_command_line("list-sessions ; new-session -d"));
    EXPECT_FALSE(parse_command_line("rename-session 'unfinished"));
    EXPECT_FALSE(parse_command_line("list-sessions\r"));
}
} // namespace tmux_cxx::protocol::tmux::control
