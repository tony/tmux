#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed pane.read_output operation and its stable native identity. */
struct ReadPaneOutput {
    /** @brief Pane identity resolved against current server state. */
    struct Request {
        PaneId pane; ///< Pane identity resolved during execution.
    };
    /** @brief Retained raw pane bytes with explicit gap errors. */
    using Response = std::string;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{6}, "pane.read_output",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::pane_raw_output)};
    /** @brief Decode the pane identity for raw output observation. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::read_pane_output */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode retained pane bytes without applying Unicode conversion. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
