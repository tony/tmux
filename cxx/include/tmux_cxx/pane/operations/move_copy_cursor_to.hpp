#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/copy_cursor_destination.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Move a copy cursor directly to a logical-line or history boundary. */
struct MoveCopyCursorTo {
    /** @brief Own a pane target and validated copy-mode arguments until server execution. */
    struct Request {
        PaneId pane;                       ///< Pane whose retained mode is addressed.
        CopyCursorDestination destination; ///< Uncounted logical-line or history boundary.
    };
    using Response = void; ///< Successful completion without a materialized viewport.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{43}, "pane.move_copy_cursor_to",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::copy_mode)};
    ///< Stable copy-mode identity and required native behavior.
    /** @brief Decode the pane and closed copy arguments before server effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::move_copy_cursor_to */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode empty success after the requested copy mutation. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
