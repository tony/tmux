#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/copy_mode_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Observe frozen copy state separately from live program terminal facts. */
struct ReadCopyMode {
    /** @brief Own a pane target until server execution. */
    struct Request {
        PaneId pane; ///< Pane whose retained mode is addressed.
    };
    using Response = CopyModeSnapshot; ///< Owned pane display freshness and optional frozen state.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{49}, "pane.copy_mode",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::copy_mode)};
    ///< Stable copy-mode identity and required native behavior.
    /** @brief Decode the pane before server effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::copy_mode */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode owner-qualified frozen state and bounded visible text. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
