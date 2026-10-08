#pragma once

#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed pane.select operation and its stable native identity. */
struct SelectPane {
    /** @brief Identify the live pane selected when the operation executes. */
    struct Request {
        PaneId pane; ///< Pane whose owning window changes selection.
    };
    using Response = PaneSnapshot; ///< Materialized selected pane after the mutation.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{20}, "pane.select",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(
            protocol::native::Capability::pane_layout)}; ///< Stable identity and requirements.
    /** @brief Decode one pane identity before observing server state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::select_pane */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the selected pane and its owning window revision. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
