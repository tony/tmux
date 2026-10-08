#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.rename operation and its stable native identity. */
struct RenameSession {
    /** @brief Session identity and replacement name resolved during execution. */
    struct Request {
        SessionId session; ///< Session identity resolved during execution.
        std::string name;  ///< Session name validated before effects.
    };
    /** @brief Successful mutation carries no result payload. */
    using Response = void;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{3}, "session.rename",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::session_names)};
    /** @brief Decode a session identity and replacement name without mutating state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::rename_session */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful session rename without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
