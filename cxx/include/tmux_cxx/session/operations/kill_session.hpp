#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.kill operation and its stable native identity. */
struct KillSession {
    /** @brief Session identity resolved when deletion executes. */
    struct Request {
        SessionId session; ///< Session identity resolved during execution.
    };
    /** @brief Successful deletion carries no result payload. */
    using Response = void;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{4}, "session.kill",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::session_lifecycle)};
    /** @brief Decode the session identity without disposing resources. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::kill_session */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful session deletion without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
