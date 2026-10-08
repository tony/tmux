#include "protocol/native/session_event_codec.hpp"

#include <concepts>
#include <limits>
#include <stdexcept>
#include <type_traits>

namespace tmux_cxx::protocol::native {
namespace {
/** @brief Assign stable native wire tags to the closed committed-event variant. */
enum class SessionEventTag : std::uint16_t {
    session_created = 1,
    session_renamed = 2,
    session_closed = 3,
};
} // namespace

void write_session_event_batch(NativePayloadWriter &writer, const SessionEventBatch &batch) {
    writer.u32(static_cast<std::uint32_t>(batch.records.size()));
    for (const auto &record : batch.records) {
        writer.u64(record.sequence);
        std::visit(
            /** @brief Encode the active committed-event alternative under its stable wire tag. */
            [&writer](const auto &event) {
                using Event = std::remove_cvref_t<decltype(event)>;
                if constexpr (std::same_as<Event, SessionCreatedEvent>) {
                    writer.u16(static_cast<std::uint16_t>(SessionEventTag::session_created));
                    writer.session(event.session);
                } else if constexpr (std::same_as<Event, SessionRenamedEvent>) {
                    writer.u16(static_cast<std::uint16_t>(SessionEventTag::session_renamed));
                    writer.u64(event.session.value);
                    writer.string(event.name);
                    writer.u64(event.revision);
                } else if constexpr (std::same_as<Event, SessionClosedEvent>) {
                    writer.u16(static_cast<std::uint16_t>(SessionEventTag::session_closed));
                    writer.u64(event.session.value);
                    writer.string(event.name);
                    writer.u64(event.revision);
                }
            },
            record.event);
    }
    writer.string(batch.next_cursor.server_instance);
    writer.u64(batch.next_cursor.next_sequence);
}

SessionEventBatch read_session_event_batch(NativePayloadReader &reader,
                                           const SessionEventCursor &requested_cursor) {
    const auto count = reader.u32();
    if (count > session_event_retention_records) {
        throw std::invalid_argument("excess session event count");
    }
    SessionEventBatch batch;
    batch.records.reserve(count);
    auto expected_sequence = requested_cursor.next_sequence;
    for (std::uint32_t index = 0; index < count; ++index) {
        const auto sequence = reader.u64();
        if (sequence != expected_sequence ||
            expected_sequence == std::numeric_limits<std::uint64_t>::max()) {
            throw std::invalid_argument("noncontiguous session event sequence");
        }
        switch (static_cast<SessionEventTag>(reader.u16())) {
        case SessionEventTag::session_created:
            batch.records.push_back({sequence, SessionCreatedEvent{reader.session()}});
            break;
        case SessionEventTag::session_renamed:
            batch.records.push_back({sequence, SessionRenamedEvent{SessionId{reader.u64()},
                                                                   reader.string(), reader.u64()}});
            break;
        case SessionEventTag::session_closed:
            batch.records.push_back({sequence, SessionClosedEvent{SessionId{reader.u64()},
                                                                  reader.string(), reader.u64()}});
            break;
        default:
            throw std::invalid_argument("unknown session event variant");
        }
        ++expected_sequence;
    }
    batch.next_cursor = {reader.string(), reader.u64()};
    if (batch.next_cursor.server_instance != requested_cursor.server_instance ||
        batch.next_cursor.next_sequence != expected_sequence) {
        throw std::invalid_argument("session event batch does not continue requested cursor");
    }
    return batch;
}
} // namespace tmux_cxx::protocol::native
