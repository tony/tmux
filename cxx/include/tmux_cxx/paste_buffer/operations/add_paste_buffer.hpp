#pragma once

#include <tmux_cxx/paste_buffer/paste_buffer_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe automatic paste-buffer creation and its stable native identity. */
struct AddPasteBuffer {
    /** @brief Own arbitrary bytes for a server-assigned automatic buffer name. */
    struct Request {
        std::string bytes; ///< Empty bytes create no buffer.
    };
    /** @brief Return created metadata, or no value for empty input. */
    using Response = std::optional<PasteBufferSnapshot>;
    /** @brief Immutable automatic-buffer mutation metadata borrowed during registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{36}, "paste_buffer.add",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers)};
    /** @brief Decode arbitrary bytes without text conversion. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::add_paste_buffer */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode whether creation occurred and its resulting metadata. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
