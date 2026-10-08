#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed client.list operation and its stable native identity. */
struct ListClients {
    /** @brief Argument-free materialized client observation. */
    struct Request {};
    /** @brief Client snapshots returned in stable identity order. */
    using Response = std::vector<ClientSnapshot>;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{13}, "client.list", protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::client_attachment)};
    /** @brief Accept an argument-free client observation request. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::clients */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode copied clients in stable identity order. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
