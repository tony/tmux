#include "pane/pane_program_input.hpp"

#include "platform/posix/errno_error.hpp"

#include <algorithm>
#include <cerrno>
#include <unistd.h>

namespace tmux_cxx {
std::size_t PaneProgramInput::remaining_capacity() const {
    return 262144 - (queued_bytes_.size() - sent_offset_);
}
bool PaneProgramInput::delivery_pending() const {
    return !admission_closed_ && sent_offset_ < queued_bytes_.size();
}
bool PaneProgramInput::accepting_input() const { return !admission_closed_; }
void PaneProgramInput::close_admission() { admission_closed_ = true; }

Result<void> PaneProgramInput::admit(std::string_view bytes) {
    if (admission_closed_) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "program input is closed"});
    }
    if (bytes.size() > remaining_capacity()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure,
                      "program input queue is full; no request bytes were admitted"});
    }
    if (sent_offset_ != 0) {
        queued_bytes_.erase(0, sent_offset_);
        sent_offset_ = 0;
    }
    queued_bytes_.append(bytes);
    return {};
}

Result<void> PaneProgramInput::flush_to_pty(int pty_descriptor, std::size_t &remaining_bytes) {
    if (admission_closed_) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "program input is closed"});
    }
    while (sent_offset_ < queued_bytes_.size() && remaining_bytes != 0) {
        const auto written_bytes =
            ::write(pty_descriptor, queued_bytes_.data() + sent_offset_,
                    std::min(queued_bytes_.size() - sent_offset_, remaining_bytes));
        if (written_bytes > 0) {
            sent_offset_ += static_cast<std::size_t>(written_bytes);
            remaining_bytes -= static_cast<std::size_t>(written_bytes);
        } else if (written_bytes < 0 && errno == EINTR) {
            continue;
        } else if (written_bytes < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            return {};
        } else {
            admission_closed_ = true;
            auto delivery_error =
                written_bytes < 0
                    ? posix::errno_error("PTY input; admitted bytes may be partially delivered")
                    : TmuxError{
                          TmuxErrorCode::io,
                          "PTY input made no progress; admitted bytes may be partially delivered"};
            return std::unexpected(std::move(delivery_error));
        }
    }
    if (sent_offset_ == queued_bytes_.size()) {
        queued_bytes_.clear();
        sent_offset_ = 0;
    }
    return {};
}
} // namespace tmux_cxx
