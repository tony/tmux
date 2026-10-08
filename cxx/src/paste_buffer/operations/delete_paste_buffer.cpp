#include <tmux_cxx/paste_buffer/operations/delete_paste_buffer.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::delete_paste_buffer(std::string_view name) {
    if (!valid_paste_buffer_name(name)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name"});
    }
    const auto buffer = state_->find_paste_buffer(name);
    if (buffer == state_->paste_buffers.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "paste buffer no longer exists"});
    }
    state_->paste_buffers.erase(buffer);
    ++state_->revision;
    return {};
}

DeletePasteBuffer::Request DeletePasteBuffer::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string()};
}

Result<DeletePasteBuffer::Response> DeletePasteBuffer::execute(Server &server, Request request) {
    return server.delete_paste_buffer(request.name);
}

protocol::native::OperationReply DeletePasteBuffer::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
