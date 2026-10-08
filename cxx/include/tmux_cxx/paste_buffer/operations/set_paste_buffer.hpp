#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe the typed paste_buffer.set operation and its stable native identity. */
struct SetPasteBuffer {
    /** @brief Carry one explicit buffer name and copied arbitrary bytes. */
    struct Request {
        std::string name;  ///< Server-global paste-buffer name.
        std::string bytes; ///< Owned bytes to store without text conversion.
    };
    /** @brief Successful storage carries no result payload. */
    using Response = void;
    /** @brief Immutable mutation metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{21}, "paste_buffer.set",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::paste_buffers)};
    /** @brief Decode a buffer name and arbitrary copied bytes. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::set_paste_buffer */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode successful storage without a result payload. */
    static protocol::native::OperationReply write();
};
} // namespace tmux_cxx
