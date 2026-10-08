#include "pane/pane_output_buffer.hpp"

#include <limits>
#include <stdexcept>

namespace tmux_cxx {
void PaneOutputBuffer::append(std::string_view bytes) {
    if (bytes.size() > std::numeric_limits<std::uint64_t>::max() - tail_offset()) {
        throw std::length_error("pane output cursor exhausted");
    }
    bytes_.append(bytes);
    if (bytes_.size() > capacity_) {
        const auto discarded = bytes_.size() - capacity_;
        bytes_.erase(0, discarded);
        begin_offset_ += discarded;
    }
}

Result<PaneOutputBuffer::Read> PaneOutputBuffer::read_from(std::uint64_t offset) const {
    if (offset < begin_offset_) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::output_gap, "pane output cursor precedes retained history"});
    }
    const auto tail = tail_offset();
    if (offset > tail) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "pane output cursor exceeds observed output"});
    }
    return Read{bytes_.substr(static_cast<std::size_t>(offset - begin_offset_)), tail};
}

std::uint64_t PaneOutputBuffer::tail_offset() const {
    return begin_offset_ + static_cast<std::uint64_t>(bytes_.size());
}

bool PaneOutputBuffer::prefix_discarded() const { return begin_offset_ != 0; }

std::string_view PaneOutputBuffer::retained_bytes() const { return bytes_; }
} // namespace tmux_cxx
