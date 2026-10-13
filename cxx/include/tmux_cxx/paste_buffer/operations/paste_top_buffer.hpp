#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe default automatic-buffer paste and its stable native identity. */
struct PasteTopBuffer {
    /** @brief Identify the live pane receiving the newest automatic buffer. */
    struct Request {
        PaneId pane; ///< Resolve before checking whether an automatic buffer exists.
    };
    /** @brief Successful admission or an empty store carries no payload. */
    using Response = void;
    /** @brief Immutable default-paste input metadata borrowed during registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{38}, "paste_buffer.paste_top",
        protocol::native::OperationEffect::program_input,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers) |
            protocol::native::capability_mask(protocol::native::Capability::pane_input)};
    /** @brief Decode the destination pane identity. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::paste_top_buffer */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful admission without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
