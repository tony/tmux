#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed client.resize operation and its stable native identity. */
struct ResizeClient {
    /** @brief Identify the attached client whose retained TTY must be remeasured. */
    struct Request {
        ClientId client; ///< Live ordinary client resolved when resize executes.
    };
    /** @brief Materialized client state carrying the newly measured dimensions. */
    using Response = ClientSnapshot;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{16}, "client.resize",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::client_attachment)};
    /** @brief Decode a client identity without remeasuring its retained terminal. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::resize_client */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode copied client state after applying its measured dimensions. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
