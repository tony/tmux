#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed paste_buffer.delete operation and its stable native identity. */
struct DeletePasteBuffer {
    /** @brief Identify one server-global named paste buffer. */
    struct Request {
        std::string name; ///< Paste-buffer name resolved during execution.
    };
    /** @brief Successful deletion carries no result payload. */
    using Response = void;
    /** @brief Immutable mutation metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{23}, "paste_buffer.delete",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers)};
    /** @brief Decode one copied paste-buffer name. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::delete_paste_buffer */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful deletion without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
