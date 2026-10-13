#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare renaming of one shared window independently of its session memberships. */
struct RenameWindow {
    /** @brief Retain a live window identity and an explicit replacement name until execution. */
    struct Request {
        WindowId window;  ///< Shared window resolved against current server state.
        std::string name; ///< Literal UTF-8 name validated before mutation.
    };
    using Response = WindowSnapshot; ///< Copied window state after the rename.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{31}, "window.rename",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::window_names)};
    ///< Stable native identity and declared window-name capability.
    /** @brief Decode the shared-window identity and literal name before any mutation. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::rename_window */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the resulting shared-window snapshot. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
