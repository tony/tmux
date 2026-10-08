#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_event.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.wait_events operation and its stable native identity. */
struct WaitSessionEvents {
    /** @brief Owner-qualified cursor and bounded wait duration for one event batch. */
    struct Request {
        SessionEventCursor cursor; ///< Position from which committed events are requested.
        std::uint32_t timeout_ms;  ///< Maximum wait in milliseconds before an empty batch returns.
    };
    /** @brief Immediate event bytes or an operation-owned readiness continuation. */
    using Response = protocol::native::OperationReply;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{27}, "session.wait_events",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::metadata_observation)};
    /** @brief Decode one cursor and wait duration before server access. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Complete on retained events or return an empty batch at the bounded deadline. */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Preserve the immediate or pending reply selected by execution. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
