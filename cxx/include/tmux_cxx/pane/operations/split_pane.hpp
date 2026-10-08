#pragma once

#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed pane.split operation and its stable native identity. */
struct SplitPane {
    /** @brief Target pane, ordered arrangement, and copied shell command. */
    struct Request {
        PaneId pane;                      ///< Existing pane divided by the operation.
        PaneSplitOrientation orientation; ///< Ordered layout arrangement for both panes.
        std::string command;              ///< Shell command started in the new pane.
    };
    using Response = PaneSnapshot; ///< Materialized new pane after layout selection.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{18}, "pane.split",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(
            protocol::native::Capability::pane_layout)}; ///< Stable identity and requirements.
    /** @brief Decode the target, closed orientation, and owned command before effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::split_pane */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode the newly selected pane and its committed geometry. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
