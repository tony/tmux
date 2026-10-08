#include "session/session_event_journal.hpp"

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
SessionEventJournal::SessionEventJournal(std::size_t capacity) : capacity_(capacity) {
    if (capacity_ == 0) {
        throw std::invalid_argument("session event retention must be nonzero");
    }
}

void SessionEventJournal::append(SessionEvent event) {
    records_.push_back(SessionEventRecord{next_sequence_++, std::move(event)});
    if (records_.size() > capacity_) {
        records_.pop_front();
    }
}

std::uint64_t SessionEventJournal::tail_sequence() const { return next_sequence_; }

Result<std::vector<SessionEventRecord>>
SessionEventJournal::read_from(std::uint64_t sequence) const {
    if (sequence > next_sequence_) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "session event cursor exceeds the journal tail"});
    }
    if (!records_.empty() && sequence < records_.front().sequence) {
        return std::unexpected(TmuxError{TmuxErrorCode::observation_gap,
                                         "session event cursor precedes retained history"});
    }
    std::vector<SessionEventRecord> records;
    records.reserve(static_cast<std::size_t>(next_sequence_ - sequence));
    for (const auto &record : records_) {
        if (record.sequence >= sequence) {
            records.push_back(record);
        }
    }
    return records;
}
} // namespace tmux_cxx
