#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare observation of shared window state independently of its session memberships. */
struct ReadWindow {
    /** @brief Resolve this shared window against current server state. */
    struct Request {
        WindowId window; ///< Shared window identity to observe.
    };
    using Response = WindowSnapshot; ///< Materialized shared window state.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{11}, "window.read", protocol::native::OperationEffect::query,
        protocol::native::capability_mask(
            protocol::native::Capability::window_links)}; ///< Stable native identity and effects.
    /** @brief Decode a shared window identity without changing its selection or membership. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode a copied shared-window snapshot. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
