#include <tmux_cxx/paste_buffer/operations/paste_buffer_into_pane.hpp>

#include "paste_buffer/paste_buffer.hpp"
#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <algorithm>

namespace tmux_cxx {
Result<void> Server::paste_buffer_into_pane(std::string_view name, PaneId pane_id) {
    const auto pane = state_->panes.find(pane_id);
    if (pane == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    if (pane->second->pty.get() < 0) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "pane PTY is closed"});
    }
    if (!valid_paste_buffer_name(name)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name"});
    }
    const auto buffer = state_->find_paste_buffer(name);
    if (buffer == state_->paste_buffers.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "paste buffer no longer exists"});
    }
    auto input = buffer->bytes;
    std::ranges::replace(input, '\n', '\r');
    return pane->second->admit_program_input(input);
}

PasteBufferIntoPane::Request
PasteBufferIntoPane::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string(), PaneId{reader.u64()}};
}

Result<PasteBufferIntoPane::Response> PasteBufferIntoPane::execute(Server &server,
                                                                   Request request) {
    return server.paste_buffer_into_pane(request.name, request.pane);
}

protocol::native::OperationReply PasteBufferIntoPane::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
