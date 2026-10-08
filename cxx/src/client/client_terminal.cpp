#include "client/client_terminal.hpp"

#include "platform/posix/errno_error.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <fcntl.h>
#include <optional>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>
#include <utility>

namespace tmux_cxx {
ClientTerminal::ClientTerminal(posix::OwnedFd input, posix::OwnedFd output, termios saved_mode,
                               int input_flags, int output_flags, terminal::CellSize size,
                               ClientTerminalSizeOrigin size_origin)
    : input_(std::move(input)), output_(std::move(output)), saved_mode_(saved_mode),
      saved_input_flags_(input_flags), saved_output_flags_(output_flags), size_(size),
      size_origin_(size_origin) {}

ClientTerminal::~ClientTerminal() { (void)restore(); }

Result<std::unique_ptr<ClientTerminal>> ClientTerminal::adopt(posix::OwnedFd input,
                                                              posix::OwnedFd output) {
    if (input.get() < 0 || output.get() < 0 || !::isatty(input.get()) || !::isatty(output.get())) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "client identification has no usable terminal"});
    }
    struct stat input_identity{};
    struct stat output_identity{};
    if (::fstat(input.get(), &input_identity) < 0 || ::fstat(output.get(), &output_identity) < 0) {
        return std::unexpected(posix::errno_error("client terminal identity"));
    }
    if (input_identity.st_rdev != output_identity.st_rdev ||
        input_identity.st_ino != output_identity.st_ino) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "client terminal routes do not match"});
    }
    termios saved_mode{};
    if (::tcgetattr(input.get(), &saved_mode) < 0) {
        return std::unexpected(posix::errno_error("client terminal attributes"));
    }
    winsize measured_size{};
    if (::ioctl(output.get(), TIOCGWINSZ, &measured_size) < 0) {
        return std::unexpected(posix::errno_error("client terminal size"));
    }
    const auto rows = measured_size.ws_row == 0 ? 24U : measured_size.ws_row;
    const auto columns = measured_size.ws_col == 0 ? 80U : measured_size.ws_col;
    const auto size_origin = measured_size.ws_row == 0 || measured_size.ws_col == 0
                                 ? ClientTerminalSizeOrigin::zero_dimension_defaults
                                 : ClientTerminalSizeOrigin::measured;
    if (rows > 512 || columns > 512 || static_cast<std::uint64_t>(rows) * columns > 16384) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "client terminal dimensions exceed supported bounds"});
    }
    const auto input_flags = ::fcntl(input.get(), F_GETFL);
    const auto output_flags = ::fcntl(output.get(), F_GETFL);
    if (input_flags < 0 || output_flags < 0 || ::fcntl(input.get(), F_SETFD, FD_CLOEXEC) < 0 ||
        ::fcntl(output.get(), F_SETFD, FD_CLOEXEC) < 0) {
        return std::unexpected(posix::errno_error("client terminal descriptor flags"));
    }
    return std::unique_ptr<ClientTerminal>(new ClientTerminal(
        std::move(input), std::move(output), saved_mode, input_flags, output_flags,
        terminal::CellSize{static_cast<std::uint16_t>(rows), static_cast<std::uint16_t>(columns)},
        size_origin));
}

Result<void> ClientTerminal::activate() {
    if (active_) {
        return {};
    }
    auto raw_mode = saved_mode_;
    raw_mode.c_iflag &= ~(IGNBRK | BRKINT | PARMRK | ISTRIP | INLCR | IGNCR | ICRNL | IXON);
    raw_mode.c_oflag &= ~OPOST;
    raw_mode.c_lflag &= ~(ECHO | ECHONL | ICANON | ISIG | IEXTEN);
    raw_mode.c_cflag &= ~(CSIZE | PARENB);
    raw_mode.c_cflag |= CS8;
    raw_mode.c_cc[VMIN] = 1;
    raw_mode.c_cc[VTIME] = 0;
    if (::fcntl(input_.get(), F_SETFL, saved_input_flags_ | O_NONBLOCK) < 0) {
        return std::unexpected(posix::errno_error("client terminal nonblocking mode"));
    }
    if (::fcntl(output_.get(), F_SETFL, saved_output_flags_ | O_NONBLOCK) < 0) {
        (void)::fcntl(input_.get(), F_SETFL, saved_input_flags_);
        return std::unexpected(posix::errno_error("client terminal nonblocking mode"));
    }
    if (::tcsetattr(input_.get(), TCSANOW, &raw_mode) < 0) {
        (void)::fcntl(input_.get(), F_SETFL, saved_input_flags_);
        (void)::fcntl(output_.get(), F_SETFL, saved_output_flags_);
        return std::unexpected(posix::errno_error("client terminal raw mode"));
    }
    active_ = true;
    return {};
}

Result<void> ClientTerminal::restore() {
    if (!active_) {
        return {};
    }
    output_bytes_.clear();
    output_sent_ = 0;
    std::optional<TmuxError> first_restoration_error;
    /** @brief Retain the first restoration failure while continuing owned cleanup. */
    const auto record_restoration_error = [&first_restoration_error](std::string_view action) {
        if (!first_restoration_error) {
            first_restoration_error = posix::errno_error(action);
        }
    };
    if (view_active_) {
        constexpr std::string_view leave_view{"\x1b[0m\x1b[?25h\x1b[?1049l"};
        std::size_t teardown_sent = 0;
        while (teardown_sent < leave_view.size()) {
            const auto written_bytes = ::write(output_.get(), leave_view.data() + teardown_sent,
                                               leave_view.size() - teardown_sent);
            if (written_bytes > 0) {
                teardown_sent += static_cast<std::size_t>(written_bytes);
            } else if (written_bytes < 0 && errno == EINTR) {
                continue;
            } else {
                record_restoration_error("client terminal view restoration");
                break;
            }
        }
        view_active_ = false;
    }
    const bool mode_restored = ::tcsetattr(input_.get(), TCSANOW, &saved_mode_) == 0;
    if (!mode_restored) {
        record_restoration_error("client terminal restoration");
    }
    const bool input_flags_restored = ::fcntl(input_.get(), F_SETFL, saved_input_flags_) == 0;
    if (!input_flags_restored) {
        record_restoration_error("client terminal input-flag restoration");
    }
    const bool output_flags_restored = ::fcntl(output_.get(), F_SETFL, saved_output_flags_) == 0;
    if (!output_flags_restored) {
        record_restoration_error("client terminal output-flag restoration");
    }
    active_ = !(mode_restored && input_flags_restored && output_flags_restored);
    if (first_restoration_error) {
        return std::unexpected(std::move(*first_restoration_error));
    }
    return {};
}

Result<void> ClientTerminal::queue_output(std::string view_bytes) {
    constexpr std::size_t output_queue_capacity = 1048576;
    const auto pending_bytes = output_bytes_.size() - output_sent_;
    if (view_bytes.size() > output_queue_capacity - pending_bytes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "client terminal output queue is full"});
    }
    if (output_sent_ != 0) {
        output_bytes_.erase(0, output_sent_);
        output_sent_ = 0;
    }
    view_active_ |= !view_bytes.empty();
    output_bytes_.append(std::move(view_bytes));
    return {};
}

Result<void> ClientTerminal::flush_output(std::size_t &remaining_output_bytes) {
    while (output_sent_ < output_bytes_.size() && remaining_output_bytes > 0) {
        const auto written_bytes =
            ::write(output_.get(), output_bytes_.data() + output_sent_,
                    std::min(remaining_output_bytes, output_bytes_.size() - output_sent_));
        if (written_bytes > 0) {
            output_sent_ += static_cast<std::size_t>(written_bytes);
            remaining_output_bytes -= static_cast<std::size_t>(written_bytes);
        } else if (written_bytes < 0 && errno == EINTR) {
            continue;
        } else if (written_bytes < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            break;
        } else {
            return std::unexpected(posix::errno_error("client terminal output"));
        }
    }
    if (output_sent_ == output_bytes_.size()) {
        output_bytes_.clear();
        output_sent_ = 0;
    }
    return {};
}

Result<ClientTerminalInput> ClientTerminal::read_input(std::size_t limit) {
    if (limit == 0) {
        return ClientTerminalInput{{}, false};
    }
    std::array<char, 8192> input_buffer{};
    for (;;) {
        const auto read_bytes =
            ::read(input_.get(), input_buffer.data(), std::min(limit, input_buffer.size()));
        if (read_bytes > 0) {
            return ClientTerminalInput{
                std::string(input_buffer.data(), static_cast<std::size_t>(read_bytes)), false};
        }
        if (read_bytes == 0) {
            return ClientTerminalInput{{}, true};
        }
        if (errno == EINTR) {
            continue;
        }
        if (errno == EAGAIN || errno == EWOULDBLOCK) {
            return ClientTerminalInput{{}, false};
        }
        return std::unexpected(posix::errno_error("client terminal input"));
    }
}

bool ClientTerminal::output_pending() const { return output_sent_ < output_bytes_.size(); }
bool ClientTerminal::active() const { return active_; }

terminal::CellSize ClientTerminal::size() const { return size_; }
ClientTerminalSizeOrigin ClientTerminal::size_origin() const { return size_origin_; }
Result<terminal::CellSize> ClientTerminal::measure() {
    winsize measured_size{};
    if (::ioctl(output_.get(), TIOCGWINSZ, &measured_size) < 0) {
        return std::unexpected(posix::errno_error("client terminal size"));
    }
    const auto rows = measured_size.ws_row == 0 ? size_.rows : measured_size.ws_row;
    const auto columns = measured_size.ws_col == 0 ? size_.columns : measured_size.ws_col;
    if (rows == 0 || columns == 0 || rows > 512 || columns > 512 ||
        static_cast<std::uint64_t>(rows) * columns > 16384) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "client terminal dimensions exceed supported bounds"});
    }
    size_ = {static_cast<std::uint16_t>(rows), static_cast<std::uint16_t>(columns)};
    if (measured_size.ws_row != 0 && measured_size.ws_col != 0) {
        size_origin_ = ClientTerminalSizeOrigin::measured;
    }
    return size_;
}
int ClientTerminal::input_descriptor() const { return input_.get(); }
int ClientTerminal::output_descriptor() const { return output_.get(); }
} // namespace tmux_cxx
