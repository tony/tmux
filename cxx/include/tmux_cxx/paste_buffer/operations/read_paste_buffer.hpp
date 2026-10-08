#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed paste_buffer.read operation and its stable native identity. */
struct ReadPasteBuffer {
    /** @brief Identify one server-global named paste buffer. */
    struct Request {
        std::string name; ///< Paste-buffer name resolved during execution.
    };
    /** @brief Copy arbitrary stored bytes to the caller. */
    using Response = std::string;
    /** @brief Immutable query metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{22}, "paste_buffer.read",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers)};
    /** @brief Decode one copied paste-buffer name. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::read_paste_buffer */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode arbitrary stored bytes as one counted native string. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
