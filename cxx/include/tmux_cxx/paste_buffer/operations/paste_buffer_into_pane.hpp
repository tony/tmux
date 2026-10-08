#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe paste_buffer.paste_into_pane and its stable native identity. */
struct PasteBufferIntoPane {
    /** @brief Identify one named buffer and its destination pane. */
    struct Request {
        std::string name; ///< Paste-buffer name resolved during execution.
        PaneId pane;      ///< Live pane receiving transformed program input.
    };
    /** @brief Successful input admission carries no result payload. */
    using Response = void;
    /** @brief Immutable program-input metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{24}, "paste_buffer.paste_into_pane",
        protocol::native::OperationEffect::program_input,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers) |
            protocol::native::capability_mask(protocol::native::Capability::pane_input)};
    /** @brief Decode a copied buffer name and destination pane identity. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::paste_buffer_into_pane */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful input admission without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
