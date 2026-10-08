#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed client.detach operation and its stable native identity. */
struct DetachClient {
    /** @brief Identify the live client whose attachment should end. */
    struct Request {
        ClientId client; ///< Live client resolved when detachment executes.
    };
    /** @brief Materialized detached client state after any terminal restoration. */
    using Response = ClientSnapshot;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{15}, "client.detach",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::client_attachment)};
    /** @brief Decode a client identity without changing attachment state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::detach_client */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode copied client state after successful detachment. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
