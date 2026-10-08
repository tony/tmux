#include <tmux_cxx/paste_buffer/operations/read_paste_buffer.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::string> Server::read_paste_buffer(std::string_view name) const {
    if (!valid_paste_buffer_name(name)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name"});
    }
    const auto buffer = state_->find_paste_buffer(name);
    if (buffer == state_->paste_buffers.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "paste buffer no longer exists"});
    }
    return buffer->bytes;
}

ReadPasteBuffer::Request ReadPasteBuffer::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string()};
}

Result<ReadPasteBuffer::Response> ReadPasteBuffer::execute(Server &server, Request request) {
    return server.read_paste_buffer(request.name);
}

protocol::native::OperationReply ReadPasteBuffer::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.string(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
