#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Begin character selection at the frozen copy cursor. */
struct BeginCopySelection {
    /** @brief Own a pane target until server execution. */
    struct Request {
        PaneId pane; ///< Pane whose retained mode is addressed.
    };
    using Response = void; ///< Successful completion without a materialized viewport.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{45}, "pane.begin_copy_selection",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::copy_mode)};
    ///< Stable copy-mode identity and required native behavior.
    /** @brief Decode the pane before server effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::begin_copy_selection */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode empty success after the requested copy mutation. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
