#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed client.attach operation and its stable native identity. */
struct AttachClient {
    /** @brief Identify the live client relationship and retained target session. */
    struct Request {
        ClientId client;   ///< Live client whose terminal or control route becomes attached.
        SessionId session; ///< Existing retained session selected for this client.
    };
    /** @brief Materialized attachment state after route-specific activation. */
    using Response = ClientSnapshot;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{14}, "client.attach",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::client_attachment)};
    /** @brief Decode client and session identities without changing attachment state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Attach an identified route, activating and sizing an ordinary terminal when present.
     */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode copied client attachment state for native delivery. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
