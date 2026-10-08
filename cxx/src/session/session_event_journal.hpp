#pragma once

#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <cstddef>
#include <deque>

namespace tmux_cxx {
/** @brief Retain a bounded sequence of committed session facts for independent observers. */
class SessionEventJournal {
  public:
    /** @brief Own copied event records under a nonzero retention bound. */
    explicit SessionEventJournal(std::size_t capacity = session_event_retention_records);
    /** @brief Copy one committed event into the next absolute sequence position. */
    void append(SessionEvent event);
    /** @brief Return the next sequence that a newly established observer should consume. */
    std::uint64_t tail_sequence() const;
    /** @brief Copy retained events from one sequence or report a discarded observation gap. */
    Result<std::vector<SessionEventRecord>> read_from(std::uint64_t sequence) const;

  private:
    std::size_t capacity_;                   ///< Maximum retained committed event records.
    std::uint64_t next_sequence_{};          ///< Sequence assigned to the next committed event.
    std::deque<SessionEventRecord> records_; ///< Retained suffix in absolute sequence order.
};
} // namespace tmux_cxx
