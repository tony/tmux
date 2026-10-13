#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare selection of a shared window through one session-owned membership. */
struct SelectWindow {
    /** @brief Resolve this membership against the server's current graph at execution. */
    struct Request {
        WindowLinkId window_link; ///< Membership whose owning session selects its shared window.
    };
    using Response =
        SessionSnapshot; ///< Owning session with its resulting window and pane selection.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{30}, "window.select",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::window_lifecycle)};
    ///< Stable native identity, mutation effects, and required window behavior.
    /** @brief Decode one session-owned window membership identity. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::select_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the owning session's resulting selection. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
