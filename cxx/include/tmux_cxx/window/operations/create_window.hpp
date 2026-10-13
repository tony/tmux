#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare creation of a shared window, its first membership, and one owned PTY program. */
struct CreateWindow {
    /** @brief Create a named window at an unused session index without selecting it. */
    struct Request {
        SessionId session;   ///< Session owning the first membership.
        std::uint32_t index; ///< Unused index bounded by the signed tmux index range.
        std::string name;    ///< Explicit UTF-8 window name, independent of session naming rules.
        std::string command; ///< Nonempty shell command for the first pane.
    };
    using Response =
        WindowLinkSnapshot; ///< Materialized first membership retaining the new window.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{29}, "window.create",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::window_lifecycle)};
    ///< Stable native identity, mutation effects, and required window behavior.
    /** @brief Decode the destination, index, name, and program command before creation effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::create_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the new window's first membership snapshot. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
