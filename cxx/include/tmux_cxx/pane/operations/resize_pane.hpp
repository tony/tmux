#pragma once

#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/resize_direction.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe pane.resize and its stable native identity. */
struct ResizePane {
    /** @brief Identify a pane border, movement direction, and maximum cell transfer. */
    struct Request {
        PaneId pane;                   ///< Target pane in a shared window.
        PaneResizeDirection direction; ///< Direction of border movement.
        std::uint32_t cells;           ///< Requested cell count; zero preserves geometry.
    };
    using Response = PaneSnapshot; ///< Copied target geometry and committed window revision.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{34}, "pane.resize",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(
            protocol::native::Capability::pane_layout)}; ///< Stable identity and requirements.
    /** @brief Decode a target, closed direction, and unsigned cell count before effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::resize_pane */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the target pane's committed geometry without exposing layout ownership. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
