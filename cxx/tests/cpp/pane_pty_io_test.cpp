#include "pane/pane.hpp"
#include "pane/pane_output_buffer.hpp"
#include "pane/pane_output_reader.hpp"
#include "platform/posix/owned_fd.hpp"

#include <tmux_cxx/server/server.hpp>

#include <array>
#include <cerrno>
#include <gtest/gtest.h>
#include <stdexcept>
#include <sys/socket.h>
#include <unistd.h>

namespace tmux_cxx {
namespace {
/** @brief Own a pane descriptor and a nonblocking peer with controlled socket buffering. */
struct ConnectedPane {
    Pane pane;           ///< Pane I/O state tested independently of program ownership.
    posix::OwnedFd peer; ///< Owned program-side socket controlling read and write boundaries.
    /** @brief Establish stream or packet boundaries without starting a program. */
    explicit ConnectedPane(int type) : pane(PaneId{1}, WindowId{1}, {24, 80}) {
        std::array<int, 2> descriptors{};
        if (::socketpair(AF_UNIX, type | SOCK_NONBLOCK | SOCK_CLOEXEC, 0, descriptors.data()) < 0) {
            throw std::runtime_error("cannot establish pane I/O fixture");
        }
        pane.pty.reset(descriptors[0]);
        peer.reset(descriptors[1]);
        const int capacity = 262144;
        if (::setsockopt(pane.pty.get(), SOL_SOCKET, SO_SNDBUF, &capacity, sizeof(capacity)) < 0 ||
            ::setsockopt(peer.get(), SOL_SOCKET, SO_SNDBUF, &capacity, sizeof(capacity)) < 0) {
            throw std::runtime_error("cannot set pane I/O fixture capacity");
        }
    }
    /** @brief Collect immediately readable program input without waiting for additional bytes. */
    std::string received_bytes() const {
        std::string received;
        std::array<char, 8192> buffer{};
        for (;;) {
            const auto count = ::read(peer.get(), buffer.data(), buffer.size());
            if (count > 0) {
                received.append(buffer.data(), static_cast<std::size_t>(count));
            } else if (count == 0 || errno == EAGAIN || errno == EWOULDBLOCK) {
                return received;
            } else if (errno != EINTR) {
                throw std::runtime_error("cannot collect pane I/O fixture input");
            }
        }
    }
};
} // namespace

/** @brief Absolute pane-output cursors distinguish retained suffixes from discarded history. */
TEST(PaneOutputBuffer, ReadsFromStableCursorAndReportsGap) {
    PaneOutputBuffer output(4);
    output.append("abc");
    const auto cursor = output.tail_offset();
    output.append("def");

    const auto suffix = output.read_from(cursor);
    ASSERT_TRUE(suffix);
    EXPECT_EQ(suffix->bytes, "def");
    EXPECT_EQ(suffix->next_offset, 6U);
    const auto discarded = output.read_from(0);
    ASSERT_FALSE(discarded);
    EXPECT_EQ(discarded.error().code, TmuxErrorCode::output_gap);
}

/** @brief Pane-output cursors reject a different server even when pane identities coincide. */
TEST(PaneOutputReader, RejectsCursorFromDifferentServer) {
    Server first;
    Server second;
    const auto first_session = first.create_session("first", "/bin/cat");
    const auto second_session = second.create_session("second", "/bin/cat");
    ASSERT_TRUE(first_session);
    ASSERT_TRUE(second_session);
    ASSERT_EQ(first_session->pane, second_session->pane);
    const auto cursor = PaneOutputReader::tail(first, first_session->pane);
    ASSERT_TRUE(cursor);

    const auto wrong_server = PaneOutputReader::read(second, *cursor);
    ASSERT_FALSE(wrong_server);
    EXPECT_EQ(wrong_server.error().code, TmuxErrorCode::wrong_server);
}

/** @brief Controlled short reads cannot consume bytes beyond one pane's output budget. */
TEST(PanePtyIo, ShortReadsRespectOutputBudget) {
    ConnectedPane connection(SOCK_SEQPACKET);
    const std::string fragment(8192, 'x');
    for (int i = 0; i < 7; ++i) {
        ASSERT_EQ(::write(connection.peer.get(), fragment.data(), fragment.size()), 8192);
    }
    ASSERT_EQ(::write(connection.peer.get(), fragment.data(), fragment.size() - 1), 8191);
    ASSERT_EQ(::write(connection.peer.get(), fragment.data(), fragment.size()), 8192);
    connection.pane.advance_pty_io();
    EXPECT_EQ(connection.pane.output.retained_bytes().size(), 65536);
}

/** @brief Admitting terminal replies cannot reset the current pane-servicing write budget. */
TEST(PanePtyIo, TerminalRepliesShareInputBudget) {
    ConnectedPane connection(SOCK_STREAM);
    const std::string input(131072, 'x');
    ASSERT_TRUE(connection.pane.program_input.admit(input));
    connection.pane.terminal_replies = "terminal-reply";
    connection.pane.advance_pty_io();
    const auto delivered = connection.received_bytes();
    EXPECT_EQ(delivered.size(), 65536);
    EXPECT_TRUE(delivered == input.substr(0, 65536));
    EXPECT_TRUE(connection.pane.program_input.delivery_pending());
}

/** @brief Forced EAGAIN preserves exact patterned bytes through successive partial input writes. */
TEST(PaneProgramInput, PartialWritesPreserveEveryByte) {
    ConnectedPane connection(SOCK_STREAM);
    const int capacity = 4096;
    ASSERT_EQ(
        ::setsockopt(connection.pane.pty.get(), SOL_SOCKET, SO_SNDBUF, &capacity, sizeof(capacity)),
        0);
    std::string admitted(131071, '\0');
    for (std::size_t i = 0; i < admitted.size(); ++i) {
        admitted[i] = static_cast<char>((i * 31 + i / 251) % 256);
    }
    ASSERT_TRUE(connection.pane.admit_program_input(admitted));
    std::size_t remaining = 65536;
    ASSERT_TRUE(connection.pane.program_input.flush_to_pty(connection.pane.pty.get(), remaining));
    ASSERT_TRUE(connection.pane.program_input.delivery_pending());
    ASSERT_NE(remaining, 0);
    std::string delivered = connection.received_bytes();
    for (int pass = 0; pass < 32 && connection.pane.program_input.delivery_pending(); ++pass) {
        remaining = 65536;
        ASSERT_TRUE(
            connection.pane.program_input.flush_to_pty(connection.pane.pty.get(), remaining));
        delivered += connection.received_bytes();
    }
    ASSERT_FALSE(connection.pane.program_input.delivery_pending());
    EXPECT_EQ(delivered.size(), admitted.size());
    EXPECT_TRUE(delivered == admitted);
}

/** @brief A full queue rejects later user bytes until earlier terminal replies can be admitted. */
TEST(PanePtyIo, QueuePressurePreservesTerminalReplyOrder) {
    ConnectedPane connection(SOCK_STREAM);
    std::string earlier(262144, 'x');
    for (std::size_t i = 0; i < earlier.size(); ++i) {
        earlier[i] = static_cast<char>('a' + (i / 16 + i % 16) % 26);
    }
    ASSERT_TRUE(connection.pane.admit_program_input(earlier));
    ASSERT_TRUE(connection.pane.terminal.ingest_program_output("\033[6n"));
    const auto reply = connection.pane.terminal.take_program_replies();
    ASSERT_EQ(reply, "\033[1;1R");
    connection.pane.terminal_replies = reply;
    const auto rejected = connection.pane.admit_program_input("later");
    ASSERT_FALSE(rejected);
    EXPECT_EQ(rejected.error().code, TmuxErrorCode::backpressure);
    connection.pane.advance_pty_io();
    std::string delivered = connection.received_bytes();
    ASSERT_TRUE(connection.pane.terminal_replies.empty());
    ASSERT_TRUE(connection.pane.admit_program_input("later"));
    for (int pass = 0; pass < 8 && connection.pane.program_input.delivery_pending(); ++pass) {
        connection.pane.advance_pty_io();
        delivered += connection.received_bytes();
    }
    ASSERT_FALSE(connection.pane.program_input.delivery_pending());
    const auto expected = earlier + reply + "later";
    EXPECT_EQ(delivered.size(), expected.size());
    EXPECT_TRUE(delivered == expected);
}

/** @brief Output EOF closes input admission even when no write was waiting to discover closure. */
TEST(PanePtyIo, OutputEofClosesInputAdmission) {
    ConnectedPane connection(SOCK_STREAM);
    connection.peer.reset();
    connection.pane.advance_pty_io();
    EXPECT_EQ(connection.pane.pty.get(), -1);
    EXPECT_FALSE(connection.pane.program_input.accepting_input());
    const auto closed = connection.pane.admit_program_input("late");
    ASSERT_FALSE(closed);
    EXPECT_EQ(closed.error().code, TmuxErrorCode::closed);
}
} // namespace tmux_cxx
