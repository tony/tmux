#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed session.list operation and its stable native identity. */
struct ListSessions {
    /** @brief Argument-free materialized session observation. */
    struct Request {};
    /** @brief Session snapshots returned in stable identity order. */
    using Response = std::vector<SessionSnapshot>;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{2}, "session.list", protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::session_lifecycle)};
    /** @brief Accept an argument-free session observation request. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::sessions */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode session snapshots in stable identity order. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
