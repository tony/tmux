#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

#include <cstddef>
#include <cstdint>
#include <string>
#include <variant>
#include <vector>

namespace tmux_cxx {
/** @brief Record one fully materialized session creation after its entity graph is consistent. */
struct SessionCreatedEvent {
    /** @brief Created session value at the committed revision. */
    SessionSnapshot session;
};

/** @brief Record one committed session-name change. */
struct SessionRenamedEvent {
    /** @brief Renamed session identity. */
    SessionId session;
    /** @brief Replacement name after validation. */
    std::string name;
    /** @brief Server mutation revision that committed the name. */
    std::uint64_t revision;
};

/** @brief Preserve a removed session's final identity and name for later observers. */
struct SessionClosedEvent {
    /** @brief Removed session identity. */
    SessionId session;
    /** @brief Final session name before removal. */
    std::string name;
    /** @brief Server mutation revision that committed removal. */
    std::uint64_t revision;
};

inline constexpr std::size_t session_event_retention_records =
    4096; ///< Maximum retained committed session-event records.

/** @brief Enumerate committed session facts without making the journal authoritative state. */
using SessionEvent = std::variant<SessionCreatedEvent, SessionRenamedEvent, SessionClosedEvent>;

/** @brief Qualify one next-session-event position by its owning server lifetime. */
struct SessionEventCursor {
    /** @brief Server lifetime that issued this cursor. */
    std::string server_instance;
    /** @brief Absolute sequence of the next session event to read. */
    std::uint64_t next_sequence;
};

/** @brief Pair one committed session event with its absolute observation sequence. */
struct SessionEventRecord {
    /** @brief Absolute sequence assigned after the session mutation commits. */
    std::uint64_t sequence;
    /** @brief Copied session fact retained independently of later state. */
    SessionEvent event;
};

/** @brief Carry copied session-event records and the cursor immediately following them. */
struct SessionEventBatch {
    /** @brief Ordered session records at or after the cursor. */
    std::vector<SessionEventRecord> records;
    /** @brief Cursor following every returned record. */
    SessionEventCursor next_cursor;
};
} // namespace tmux_cxx
