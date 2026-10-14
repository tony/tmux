#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/copy_cursor_direction.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Move a copy cursor by counted relative cell steps. */
struct MoveCopyCursor {
    /** @brief Own a pane target and validated copy-mode arguments until server execution. */
    struct Request {
        PaneId pane;                   ///< Pane whose retained mode is addressed.
        CopyCursorDirection direction; ///< Relative movement within frozen backing.
        std::uint32_t count;           ///< Requested steps; zero preserves the cursor.
    };
    using Response = void; ///< Successful completion without a materialized viewport.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{42}, "pane.move_copy_cursor",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::copy_mode)};
    ///< Stable copy-mode identity and required native behavior.
    /** @brief Decode the pane and closed copy arguments before server effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::move_copy_cursor */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode empty success after the requested copy mutation. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
