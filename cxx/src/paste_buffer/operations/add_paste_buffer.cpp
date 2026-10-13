#include <tmux_cxx/paste_buffer/operations/add_paste_buffer.hpp>

#include "paste_buffer/paste_buffer_native_codec.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::optional<PasteBufferSnapshot>> Server::add_paste_buffer(std::string bytes) {
    auto added = state_->paste_buffer_store.add(std::move(bytes), state_->revision + 1);
    if (added && added->has_value()) {
        ++state_->revision;
    }
    return added;
}

AddPasteBuffer::Request AddPasteBuffer::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string()};
}

Result<AddPasteBuffer::Response> AddPasteBuffer::execute(Server &server, Request request) {
    return server.add_paste_buffer(std::move(request.bytes));
}

protocol::native::OperationReply AddPasteBuffer::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u16(response.has_value() ? 1 : 0);
    if (response) {
        protocol::native::write_paste_buffer_snapshot(reply, *response);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
