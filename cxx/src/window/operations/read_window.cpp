#include <tmux_cxx/window/operations/read_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<WindowSnapshot> Server::window(WindowId window_id) const {
    auto window_entry = state_->windows.find(window_id);
    if (window_entry == state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    const auto &window_state = window_entry->second;
    return WindowSnapshot{window_state.id,        window_state.active,       window_state.name,
                          window_state.size.rows, window_state.size.columns, window_state.revision};
}
ReadWindow::Request ReadWindow::read(protocol::native::NativePayloadReader &reader) {
    return {WindowId{reader.u64()}};
}
Result<ReadWindow::Response> ReadWindow::execute(Server &server, Request request) {
    return server.window(request.window);
}
protocol::native::OperationReply ReadWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.window(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
