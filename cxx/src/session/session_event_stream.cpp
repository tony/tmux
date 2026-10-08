#include <tmux_cxx/session/session_event_stream.hpp>

#include "connection/native_server_channel.hpp"
#include <tmux_cxx/connection/server_connection.hpp>

#include <atomic>
#include <mutex>
#include <stop_token>
#include <utility>
#include <vector>

namespace tmux_cxx {
/** @brief Hold one stream's cursor, buffered records, and cancellation state. */
struct SessionEventStream::State {
    /** @brief Retain the shared native channel and copy this observer's atomic baseline. */
    State(std::shared_ptr<NativeServerChannel> channel_value, SessionObservation observation)
        : channel(std::move(channel_value)), initial(std::move(observation)),
          cursor(initial.next_event) {}

    std::shared_ptr<NativeServerChannel> channel; ///< Shared server identity and request admission.
    SessionObservation initial;                   ///< Immutable creation snapshot and cursor.
    SessionEventCursor cursor;                ///< Next committed record expected by this stream.
    std::vector<SessionEventRecord> buffered; ///< Received records awaiting individual delivery.
    std::size_t buffered_index = 0;           ///< Next buffered record to deliver.
    std::mutex next_mutex;                    ///< Serialize consumption of the owned cursor.
    std::stop_source cancellation;            ///< Interrupt this stream's admitted native wait.
    std::atomic<bool> closed = false;         ///< Reject work after explicit or scoped closure.
};

SessionEventStream::SessionEventStream(std::shared_ptr<NativeServerChannel> channel,
                                       SessionObservation observation)
    : state_(std::make_unique<State>(std::move(channel), std::move(observation))) {}

SessionEventStream::~SessionEventStream() { close(); }

SessionEventStream::SessionEventStream(SessionEventStream &&other) noexcept = default;

SessionEventStream &SessionEventStream::operator=(SessionEventStream &&other) noexcept {
    if (this != &other) {
        close();
        state_ = std::move(other.state_);
    }
    return *this;
}

SessionObservation SessionEventStream::initial() const {
    if (!state_) {
        return {};
    }
    return state_->initial;
}

Result<std::optional<SessionEventStreamItem>> SessionEventStream::next(std::uint32_t timeout_ms) {
    if (!state_) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::closed, "session event stream has been moved"});
    }
    std::unique_lock next_lock(state_->next_mutex);
    if (state_->closed.load()) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "session event stream is closed"});
    }
    if (state_->buffered_index < state_->buffered.size()) {
        return std::optional<SessionEventStreamItem>{
            std::move(state_->buffered[state_->buffered_index++])};
    }
    state_->buffered.clear();
    state_->buffered_index = 0;

    ServerConnection connection(state_->channel);
    auto event_batch = connection.wait_session_events_until_cancelled(
        state_->cursor, timeout_ms, state_->cancellation.get_token());
    if (!event_batch && event_batch.error().code == TmuxErrorCode::observation_gap) {
        const auto missed_from = state_->cursor;
        auto replacement =
            connection.observe_sessions_until_cancelled(state_->cancellation.get_token());
        if (!replacement) {
            return std::unexpected(replacement.error());
        }
        state_->cursor = replacement->next_event;
        return std::optional<SessionEventStreamItem>{
            SessionEventGap{missed_from, std::move(*replacement)}};
    }
    if (!event_batch) {
        return std::unexpected(event_batch.error());
    }
    state_->cursor = event_batch->next_cursor;
    if (event_batch->records.empty()) {
        return std::optional<SessionEventStreamItem>{};
    }
    state_->buffered = std::move(event_batch->records);
    state_->buffered_index = 1;
    return std::optional<SessionEventStreamItem>{std::move(state_->buffered.front())};
}

void SessionEventStream::close() noexcept {
    if (state_ && !state_->closed.exchange(true)) {
        state_->cancellation.request_stop();
    }
}

Result<SessionEventStream> ServerConnection::subscribe_sessions() {
    auto observation = observe_sessions();
    if (!observation) {
        return std::unexpected(observation.error());
    }
    return SessionEventStream(channel_, std::move(*observation));
}
} // namespace tmux_cxx
