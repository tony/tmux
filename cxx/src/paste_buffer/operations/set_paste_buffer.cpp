#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::set_paste_buffer(std::string name, std::string bytes) {
    const auto stored =
        state_->paste_buffer_store.set(std::move(name), std::move(bytes), state_->revision + 1);
    if (!stored) {
        return std::unexpected(stored.error());
    }
    if (*stored) {
        ++state_->revision;
    }
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
