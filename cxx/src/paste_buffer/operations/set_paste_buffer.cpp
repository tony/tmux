#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::set_paste_buffer(std::string name, std::string bytes) {
    if (!valid_paste_buffer_name(name) || bytes.size() > paste_buffer_byte_limit) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name or size"});
    }
    if (bytes.empty()) {
        return {};
    }
    auto existing = state_->find_paste_buffer(name);
    if (existing == state_->paste_buffers.end() &&
        state_->paste_buffers.size() == paste_buffer_count_limit) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "paste-buffer store is full"});
    }
    if (existing != state_->paste_buffers.end()) {
        state_->paste_buffers.erase(existing);
    }
    state_->paste_buffers.insert(
        state_->paste_buffers.begin(),
        PasteBuffer{std::move(name), std::move(bytes), ++state_->revision});
    return {};
}

SetPasteBuffer::Request SetPasteBuffer::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string(), reader.string()};
}

Result<SetPasteBuffer::Response> SetPasteBuffer::execute(Server &server, Request request) {
    return server.set_paste_buffer(std::move(request.name), std::move(request.bytes));
}

protocol::native::OperationReply SetPasteBuffer::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
