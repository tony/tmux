#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window/layout_preset.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare selection of a named layout for one shared window. */
struct SelectLayout {
    /** @brief Retain the live window identity and the requested tmux layout preset. */
    struct Request {
        WindowId window;           ///< Shared window resolved at execution.
        WindowLayoutPreset preset; ///< Arrangement preserving logical pane order.
    };
    using Response = WindowSnapshot; ///< Copied window state after arranging its panes.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{35}, "window.select_layout",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::pane_layout)};
    ///< Stable native identity and declared layout capability.
    /** @brief Decode a window identity and validate a closed layout preset before effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::select_layout */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the resulting shared-window snapshot. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
