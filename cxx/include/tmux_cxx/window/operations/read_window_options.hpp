#pragma once

#include <tmux_cxx/window/operations/set_window_option.hpp>
#include <tmux_cxx/window/window_options_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe observation of global defaults or one shared window's effective options. */
struct ReadWindowOptions {
    /** @brief Identify the live option owner without retaining an internal entity pointer. */
    struct Request {
        WindowOptionTarget target; ///< Global defaults or one resolved shared window.
    };
    using Response = WindowOptionsSnapshot; ///< Owned effective options and their freshness.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{40}, "window.options",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::window_options)};
    ///< Stable window-option query identity and required capability.
    /** @brief Decode one explicit global-or-window option target. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::window_options */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode owner, optional window identity, effective mode keys, and revision. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
