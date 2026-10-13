#include <tmux_cxx/paste_buffer/operations/delete_paste_buffer.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::delete_paste_buffer(std::string_view name) {
    if (auto deleted = state_->paste_buffer_store.erase(name); !deleted) {
        return deleted;
    }
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
