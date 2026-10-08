#pragma once

#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

#include <vector>

namespace tmux_cxx {
/** @brief Describe the typed pane.list operation and its stable native identity. */
struct ListPanes {
    /** @brief Identify the shared window whose layout is observed. */
    struct Request {
        WindowId window; ///< Window resolved when the query executes.
    };
    using Response = std::vector<PaneSnapshot>; ///< Materialized panes in layout order.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{19}, "pane.list", protocol::native::OperationEffect::query,
        protocol::native::capability_mask(
            protocol::native::Capability::pane_layout)}; ///< Stable identity and requirements.
    /** @brief Decode one window identity without observing server state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::panes */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode bounded pane snapshots in layout order. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
