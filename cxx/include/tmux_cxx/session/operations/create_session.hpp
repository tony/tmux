#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.create operation and its stable native identity. */
struct CreateSession {
    /** @brief Session name and shell command owned before process creation. */
    struct Request {
        std::string name;    ///< Session name validated before effects.
        std::string command; ///< Shell command executed through /bin/sh -c.
    };
    /** @brief Session snapshot retained independently of the live session. */
    using Response = SessionSnapshot;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{1}, "session.create",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::session_lifecycle)};
    /** @brief Decode a session name and shell command without creating a process. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::create_session */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the created session snapshot for native delivery. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
