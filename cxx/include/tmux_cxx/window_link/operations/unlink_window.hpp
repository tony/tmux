#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare explicit removal of one session-to-window membership. */
struct UnlinkWindow {
    /** @brief Resolve this membership against current server state at execution. */
    struct Request {
        WindowLinkId window_link; ///< Membership identity to remove.
    };
    using Response = void; ///< Successful removal has no result payload.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{9}, "window.unlink",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(
            protocol::native::Capability::window_links)}; ///< Stable native identity and effects.
    /** @brief Decode a window membership identity without removing it. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::unlink_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode completed membership removal without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
