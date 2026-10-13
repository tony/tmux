#include <tmux_cxx/paste_buffer/operations/list_paste_buffers.hpp>

#include "paste_buffer/paste_buffer_native_codec.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::vector<PasteBufferSnapshot>> Server::paste_buffers() const {
    return state_->paste_buffer_store.snapshots();
}

ListPasteBuffers::Request ListPasteBuffers::read(protocol::native::NativePayloadReader &) {
    return {};
}

Result<ListPasteBuffers::Response> ListPasteBuffers::execute(Server &server, Request) {
    return server.paste_buffers();
}

protocol::native::OperationReply ListPasteBuffers::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.size()));
    for (const auto &buffer : response) {
        protocol::native::write_paste_buffer_snapshot(reply, buffer);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
