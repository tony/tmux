#include "platform/posix/owned_fd.hpp"
#include "protocol/tmux/control/channel.hpp"
#include "protocol/tmux/control/limits.hpp"
#include "protocol/tmux/control/pane_output.hpp"
#include "protocol/tmux/control/session_changed.hpp"
#include "protocol/tmux/control/session_renamed.hpp"
#include "protocol/tmux/control/sessions_changed.hpp"

#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

#include <gtest/gtest.h>

#include <array>
#include <fcntl.h>
#include <unistd.h>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Control records preserve printable bytes and octal-escape controls and backslashes. */
TEST(ControlRecords, FormatSessionChangeAndPaneOutput) {
    const SessionSnapshot session{SessionId{3}, WindowId{4},     WindowLinkId{5},
                                  PaneId{7},    "named session", 9};
    EXPECT_EQ(format_session_changed(session), "%session-changed $3 named session\n");
    EXPECT_EQ(format_session_renamed(SessionRenamedEvent{SessionId{3}, "new name", 10}),
              "%session-renamed $3 new name\n");
    EXPECT_EQ(format_sessions_changed(), "%sessions-changed\n");
    const std::string bytes{"A\0\\\n", 4};
    EXPECT_EQ(format_pane_output(PaneId{7}, bytes), "%output %7 A\\000\\134\\012\n");
}

/** @brief A control channel preserves line order, exit, guards, and output bytes. */
TEST(ControlChannel, RoutesBoundedLinesAndGuardedOutput) {
    std::array<int, 2> input_pipe{};
    std::array<int, 2> output_pipe{};
    ASSERT_EQ(::pipe(input_pipe.data()), 0);
    ASSERT_EQ(::pipe(output_pipe.data()), 0);
    posix::OwnedFd input_writer(input_pipe[1]);
    posix::OwnedFd output_reader(output_pipe[0]);
    posix::OwnedFd input_flags_observer(::dup(input_pipe[0]));
    posix::OwnedFd output_flags_observer(::dup(output_pipe[1]));
    ASSERT_GE(input_flags_observer.get(), 0);
    ASSERT_GE(output_flags_observer.get(), 0);
    const int original_input_flags = ::fcntl(input_flags_observer.get(), F_GETFL);
    const int original_output_flags = ::fcntl(output_flags_observer.get(), F_GETFL);
    auto channel = Channel::adopt(posix::OwnedFd(input_pipe[0]), posix::OwnedFd(output_pipe[1]));
    ASSERT_TRUE(channel);
    EXPECT_NE(::fcntl(input_flags_observer.get(), F_GETFL) & O_NONBLOCK, 0);
    EXPECT_NE(::fcntl(output_flags_observer.get(), F_GETFL) & O_NONBLOCK, 0);

    constexpr std::string_view input{"rename-session -t old new\n\nignored\n"};
    ASSERT_EQ(::write(input_writer.get(), input.data(), input.size()),
              static_cast<ssize_t>(input.size()));
    auto batch = (*channel)->read_input(read_bytes_per_turn);
    ASSERT_TRUE(batch);
    EXPECT_EQ(batch->command_lines, (std::vector<std::string>{"rename-session -t old new"}));
    EXPECT_TRUE(batch->exit_requested);

    ASSERT_TRUE((*channel)->queue_command_result(stock_v8_control_mode,
                                                 {42, 7, CommandOrigin::control_input},
                                                 CommandOutcome::succeeded, "renamed"));
    ASSERT_TRUE((*channel)->queue_record("%session-changed $1 control\n"));
    std::size_t output_budget = write_bytes_per_turn;
    ASSERT_TRUE((*channel)->flush_output(output_budget));
    std::array<char, 256> output{};
    const auto output_bytes = ::read(output_reader.get(), output.data(), output.size());
    ASSERT_GT(output_bytes, 0);
    EXPECT_EQ(std::string_view(output.data(), static_cast<std::size_t>(output_bytes)),
              "%begin 42 7 1\nrenamed\n%end 42 7 1\n%session-changed $1 control\n");
    channel->reset();
    EXPECT_EQ(::fcntl(input_flags_observer.get(), F_GETFL), original_input_flags);
    EXPECT_EQ(::fcntl(output_flags_observer.get(), F_GETFL), original_output_flags);
}

/** @brief One readiness turn rejects a command burst beyond its execution bound. */
TEST(ControlChannel, RejectsExcessCommandBatch) {
    std::array<int, 2> input_pipe{};
    std::array<int, 2> output_pipe{};
    ASSERT_EQ(::pipe(input_pipe.data()), 0);
    ASSERT_EQ(::pipe(output_pipe.data()), 0);
    posix::OwnedFd input_writer(input_pipe[1]);
    posix::OwnedFd output_reader(output_pipe[0]);
    ASSERT_GE(output_reader.get(), 0);
    auto channel = Channel::adopt(posix::OwnedFd(input_pipe[0]), posix::OwnedFd(output_pipe[1]));
    ASSERT_TRUE(channel);
    std::string commands;
    for (std::size_t index = 0; index < maximum_commands_per_turn + 1; ++index) {
        commands.append("x\n");
    }
    ASSERT_EQ(::write(input_writer.get(), commands.data(), commands.size()),
              static_cast<ssize_t>(commands.size()));
    EXPECT_FALSE((*channel)->read_input(read_bytes_per_turn));
}
} // namespace tmux_cxx::protocol::tmux::control
