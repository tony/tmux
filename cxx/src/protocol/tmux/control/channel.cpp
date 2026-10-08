#include "protocol/tmux/control/channel.hpp"

#include "platform/posix/errno_error.hpp"
#include "protocol/tmux/control/limits.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <fcntl.h>
#include <unistd.h>
#include <utility>

namespace tmux_cxx::protocol::tmux::control {
Channel::Channel(posix::OwnedFd input, posix::OwnedFd output, int input_flags, int output_flags)
    : input_(std::move(input)), output_(std::move(output)), input_flags_(input_flags),
      output_flags_(output_flags) {}

Channel::~Channel() {
    if (input_.get() >= 0) {
        (void)::fcntl(input_.get(), F_SETFL, input_flags_);
    }
    if (output_.get() >= 0) {
        (void)::fcntl(output_.get(), F_SETFL, output_flags_);
    }
}

Result<std::unique_ptr<Channel>> Channel::adopt(posix::OwnedFd input, posix::OwnedFd output) {
    if (input.get() < 0 || output.get() < 0) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "control client requires input and output"});
    }
    const int input_flags = ::fcntl(input.get(), F_GETFL);
    const int output_flags = ::fcntl(output.get(), F_GETFL);
    if (input_flags < 0 || output_flags < 0) {
        return std::unexpected(posix::errno_error("control client descriptor flags"));
    }
    if (::fcntl(input.get(), F_SETFL, input_flags | O_NONBLOCK) < 0) {
        return std::unexpected(posix::errno_error("control client input nonblocking mode"));
    }
    if (::fcntl(output.get(), F_SETFL, output_flags | O_NONBLOCK) < 0) {
        (void)::fcntl(input.get(), F_SETFL, input_flags);
        return std::unexpected(posix::errno_error("control client output nonblocking mode"));
    }
    return std::unique_ptr<Channel>(
        new Channel(std::move(input), std::move(output), input_flags, output_flags));
}

Result<void> Channel::queue_command_result(const TmuxControlModeProfile &grammar,
                                           CommandGuard guard, CommandOutcome outcome,
                                           std::string_view body) {
    return queue_output_bytes(format_command_result(grammar, guard, outcome, body));
}

Result<void> Channel::queue_record(std::string record) {
    if (record.empty() || record.back() != '\n') {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "control record must end with a newline"});
    }
    return queue_output_bytes(std::move(record));
}

Result<void> Channel::queue_output_bytes(std::string bytes) {
    const auto pending_bytes = output_bytes_.size() - output_sent_;
    if (bytes.size() > maximum_output_bytes - pending_bytes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "control client output queue is full"});
    }
    if (output_sent_ != 0) {
        output_bytes_.erase(0, output_sent_);
        output_sent_ = 0;
    }
    output_bytes_.append(std::move(bytes));
    return {};
}

Result<InputBatch> Channel::read_input(std::size_t byte_budget) {
    std::array<char, 8192> input_buffer{};
    bool input_closed = false;
    while (byte_budget > 0) {
        const auto read_bytes =
            ::read(input_.get(), input_buffer.data(), std::min(byte_budget, input_buffer.size()));
        if (read_bytes > 0) {
            const auto count = static_cast<std::size_t>(read_bytes);
            if (count > maximum_line_bytes - input_bytes_.size()) {
                return std::unexpected(TmuxError{TmuxErrorCode::backpressure,
                                                 "control client input line is too long"});
            }
            input_bytes_.append(input_buffer.data(), count);
            byte_budget -= count;
        } else if (read_bytes == 0) {
            input_closed = true;
            break;
        } else if (errno == EINTR) {
            continue;
        } else if (errno == EAGAIN || errno == EWOULDBLOCK) {
            break;
        } else {
            return std::unexpected(posix::errno_error("control client input"));
        }
    }

    InputBatch batch{{}, input_closed};
    for (;;) {
        const auto line_end = input_bytes_.find('\n');
        if (line_end == std::string::npos) {
            break;
        }
        std::string line = input_bytes_.substr(0, line_end);
        input_bytes_.erase(0, line_end + 1);
        if (line.empty()) {
            batch.exit_requested = true;
            input_bytes_.clear();
            break;
        }
        if (batch.command_lines.size() == maximum_commands_per_turn) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure, "control command batch is too large"});
        }
        batch.command_lines.push_back(std::move(line));
    }
    if (batch.exit_requested) {
        input_bytes_.clear();
    }
    return batch;
}

Result<void> Channel::flush_output(std::size_t &byte_budget) {
    while (output_sent_ < output_bytes_.size() && byte_budget > 0) {
        const auto written_bytes =
            ::write(output_.get(), output_bytes_.data() + output_sent_,
                    std::min(byte_budget, output_bytes_.size() - output_sent_));
        if (written_bytes > 0) {
            const auto count = static_cast<std::size_t>(written_bytes);
            output_sent_ += count;
            byte_budget -= count;
        } else if (written_bytes < 0 && errno == EINTR) {
            continue;
        } else if (written_bytes < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            break;
        } else {
            return std::unexpected(posix::errno_error("control client output"));
        }
    }
    if (output_sent_ == output_bytes_.size()) {
        output_bytes_.clear();
        output_sent_ = 0;
    }
    return {};
}

bool Channel::output_pending() const { return output_sent_ < output_bytes_.size(); }
int Channel::input_descriptor() const { return input_.get(); }
int Channel::output_descriptor() const { return output_.get(); }
} // namespace tmux_cxx::protocol::tmux::control
