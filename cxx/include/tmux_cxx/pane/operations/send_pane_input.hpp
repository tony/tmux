#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>

namespace tmux_cxx {
/** @brief Describe the typed pane.send_input operation and its stable native identity. */
struct SendPaneInput {
    /** @brief Pane identity and copied raw program-input bytes. */
    struct Request {
        PaneId pane;       ///< Pane identity resolved during execution.
        std::string bytes; ///< Owned raw program-input bytes.
    };
    /** @brief Successful admission carries no result payload. */
    using Response = void;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{5}, "pane.send_input",
        protocol::native::OperationEffect::program_input,
        protocol::native::capability_mask(protocol::native::Capability::pane_input)};
    /** @brief Decode the pane identity and raw program-input bytes. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::send_pane_input */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful program input without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
