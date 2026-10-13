#include <tmux_cxx/paste_buffer/operations/paste_top_buffer.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::paste_top_buffer(PaneId pane_id) {
    const auto pane = state_->panes.find(pane_id);
    if (pane == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    if (pane->second->pty.get() < 0 || !pane->second->program_input.accepting_input()) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "pane program input is closed"});
    }
    const auto *buffer = state_->paste_buffer_store.top();
    return buffer == nullptr ? Result<void>{} : paste_buffer_into_pane(buffer->name, pane_id);
}

PasteTopBuffer::Request PasteTopBuffer::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}

Result<PasteTopBuffer::Response> PasteTopBuffer::execute(Server &server, Request request) {
    return server.paste_top_buffer(request.pane);
}

protocol::native::OperationReply PasteTopBuffer::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
