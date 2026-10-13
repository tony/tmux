#pragma once

#include <tmux_cxx/paste_buffer/paste_buffer_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

#include <vector>

namespace tmux_cxx {
/** @brief Describe paste-buffer enumeration and its stable native identity. */
struct ListPasteBuffers {
    /** @brief Request all retained paste-buffer metadata without a target. */
    struct Request {};
    /** @brief Materialize buffers in newest-first storage order. */
    using Response = std::vector<PasteBufferSnapshot>;
    /** @brief Immutable paste-buffer query metadata borrowed during registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{37}, "paste_buffer.list",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers)};
    /** @brief Decode the empty query payload. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::paste_buffers */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode bounded newest-first buffer metadata. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
