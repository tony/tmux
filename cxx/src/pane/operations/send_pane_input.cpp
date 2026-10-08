#include <tmux_cxx/pane/operations/send_pane_input.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::send_pane_input(PaneId pane_id, std::string_view bytes) {
    const auto pane_entry = state_->panes.find(pane_id);
    if (pane_entry == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    if (bytes.size() > 65536) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "input exceeds the request limit"});
    }
    auto &pane = *pane_entry->second;
    if (pane.pty.get() < 0) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "pane PTY is closed"});
    }
    return pane.admit_program_input(bytes);
}

SendPaneInput::Request SendPaneInput::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}, reader.string()};
}

Result<SendPaneInput::Response> SendPaneInput::execute(Server &server, Request request) {
    return server.send_pane_input(request.pane, request.bytes);
}

protocol::native::OperationReply SendPaneInput::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
