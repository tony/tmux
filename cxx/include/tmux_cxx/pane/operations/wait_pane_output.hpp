#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed pane.wait_output operation and its stable native identity. */
struct WaitPaneOutput {
    /** @brief Pane identity, copied byte pattern, and millisecond deadline. */
    struct Request {
        PaneId pane;              ///< Pane identity resolved during execution.
        std::string needle;       ///< Nonempty raw byte pattern.
        std::uint32_t timeout_ms; ///< Deadline duration in milliseconds; supported range is 1..750.
    };
    /** @brief Immediate bytes or a deadline-owned readiness continuation. */
    using Response = protocol::native::OperationReply;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{7}, "pane.wait_output",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::pane_output_wait)};
    /** @brief Decode the pane identity, byte pattern, and millisecond deadline. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @brief Await bytes or return an owned continuation without blocking the server loop. */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Transfer the immediate reply or owned continuation to the native transport. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
