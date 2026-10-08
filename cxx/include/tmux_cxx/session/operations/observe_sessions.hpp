#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_observation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.observe operation and its stable native identity. */
struct ObserveSessions {
    /** @brief Argument-free request for one session catalog observation boundary. */
    struct Request {};
    /** @brief Materialized session catalog and its following event position. */
    using Response = SessionObservation;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{25}, "session.observe",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::metadata_observation)};
    /** @brief Accept an argument-free session catalog observation request. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Materialize sessions and their following event cursor at one mutation boundary. */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the session catalog followed by its owner-qualified event cursor. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
