#pragma once

#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/session/session_observation.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <cstdint>
#include <memory>
#include <optional>
#include <variant>

namespace tmux_cxx {
class NativeServerChannel;
class ServerConnection;

/** @brief Report discarded session history together with its replacement atomic observation. */
struct SessionEventGap {
    /** @brief Cursor whose unread session-event prefix is no longer retained. */
    SessionEventCursor missed_from;
    /** @brief Current sessions and the first event following their replacement snapshot. */
    SessionObservation replacement;
};

/** @brief Deliver one committed session fact or an explicit snapshot recovery boundary. */
using SessionEventStreamItem = std::variant<SessionEventRecord, SessionEventGap>;

/** @brief Own one session observer's cursor, buffered batch, cancellation, and recovery path. */
class SessionEventStream {
  public:
    /** @brief Cancel admitted observation work without changing persistent sessions. */
    ~SessionEventStream();
    /** @brief Prevent two streams from consuming one observer-owned cursor. */
    SessionEventStream(const SessionEventStream &) = delete;
    /** @brief Prevent assigning shared cursor consumption. */
    SessionEventStream &operator=(const SessionEventStream &) = delete;
    /** @brief Transfer the observer-owned cursor and cancellation scope. */
    SessionEventStream(SessionEventStream &&other) noexcept;
    /** @brief Cancel current work before adopting another stream's observation scope. */
    SessionEventStream &operator=(SessionEventStream &&other) noexcept;

    /** @brief Copy the atomic session baseline established when this stream was created. */
    SessionObservation initial() const;
    /** @brief Return one event, gap recovery, or no item after a 1..750 millisecond wait. */
    Result<std::optional<SessionEventStreamItem>> next(std::uint32_t timeout_ms = 500);
    /** @brief Cancel an admitted wait and reject later reads while preserving server state. */
    void close() noexcept;

  private:
    struct State;
    /** @brief Start from an atomic baseline on a shared native server channel. */
    SessionEventStream(std::shared_ptr<NativeServerChannel> channel,
                       SessionObservation observation);
    friend class ServerConnection;
    std::unique_ptr<State> state_; ///< Exclusive observer cursor and cancellation state.
};
} // namespace tmux_cxx
