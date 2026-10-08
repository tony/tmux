#include <tmux_cxx/session/operations/wait_session_events.hpp>

#include "protocol/native/framing.hpp"
#include "protocol/native/session_event_codec.hpp"
#include "session/session_event_reader.hpp"

namespace tmux_cxx {
namespace {
/** @brief Encode one event batch for immediate completion or a resumed pending request. */
protocol::native::Bytes encode_event_batch(const SessionEventBatch &batch) {
    protocol::native::NativePayloadWriter reply;
    protocol::native::write_session_event_batch(reply, batch);
    return std::move(reply.bytes);
}
} // namespace

WaitSessionEvents::Request WaitSessionEvents::read(protocol::native::NativePayloadReader &reader) {
    return {{reader.string(), reader.u64()}, reader.u32()};
}

Result<WaitSessionEvents::Response> WaitSessionEvents::execute(Server &server, Request request) {
    if (request.timeout_ms == 0 || request.timeout_ms > 750) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "session event wait requires a 1..750 ms deadline"});
    }
    auto current_events = SessionEventReader::read(server, request.cursor);
    if (!current_events) {
        return std::unexpected(current_events.error());
    }
    if (!current_events->records.empty()) {
        return protocol::native::OperationReply{encode_event_batch(*current_events)};
    }
    const auto wait_deadline =
        protocol::native::Clock::now() + std::chrono::milliseconds(request.timeout_ms);
    /** @brief Resume from the same cursor until an event arrives or the owned deadline expires. */
    auto await_events =
        [cursor = std::move(request.cursor),
         wait_deadline](Server &current_server) -> Result<std::optional<protocol::native::Bytes>> {
        auto event_batch = SessionEventReader::read(current_server, cursor);
        if (!event_batch) {
            return std::unexpected(event_batch.error());
        }
        if (!event_batch->records.empty() || protocol::native::Clock::now() >= wait_deadline) {
            return std::optional<protocol::native::Bytes>{encode_event_batch(*event_batch)};
        }
        return std::optional<protocol::native::Bytes>{};
    };
    return protocol::native::OperationReply{
        protocol::native::PendingReply{wait_deadline, std::move(await_events)}};
}

protocol::native::OperationReply WaitSessionEvents::write(Response response) { return response; }
} // namespace tmux_cxx
