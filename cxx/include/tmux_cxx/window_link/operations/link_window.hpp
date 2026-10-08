#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare a window membership mutation using server-local window and session identities. */
struct LinkWindow {
    /** @brief Assign an existing window to an unused index within the destination session. */
    struct Request {
        WindowId window;     ///< Shared window to retain through a new membership.
        SessionId session;   ///< Destination session owning the index.
        std::uint32_t index; ///< Destination index, bounded by the signed tmux index range.
    };
    using Response = WindowLinkSnapshot; ///< Materialized newly established membership.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{8}, "window.link",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(
            protocol::native::Capability::window_links)}; ///< Stable native identity and effects.
    /** @brief Decode window identity, destination session, and index before any membership effects.
     */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::link_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the established window-membership snapshot. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
