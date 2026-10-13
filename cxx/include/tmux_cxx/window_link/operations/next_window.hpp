#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare next window selection through the session's ordered memberships. */
struct NextWindow {
    /** @brief Resolve one session's current membership at execution. */
    struct Request {
        SessionId session; ///< Session whose own selection advances.
    };
    using Response = SessionSnapshot; ///< Copied session state after selection.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{32}, "window.next",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::window_lifecycle)};
    ///< Stable native identity and window-selection requirement.
    /** @brief Decode the session identity before resolving its current membership. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::next_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the resulting session selection. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
