#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_event.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.read_events operation and its stable native identity. */
struct ReadSessionEvents {
    /** @brief Owner-qualified next-event position supplied by one observer. */
    struct Request {
        SessionEventCursor cursor; ///< Position from which retained committed events are requested.
    };
    /** @brief Ordered retained events and the position following the returned batch. */
    using Response = SessionEventBatch;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{26}, "session.read_events",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::metadata_observation)};
    /** @brief Decode one owner-qualified event cursor before server access. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Read retained committed events or preserve ownership and gap failures. */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode ordered variant records and their following cursor. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
