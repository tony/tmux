#include <tmux_cxx/window/operations/rename_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "window/window_name.hpp"

#include <utility>

namespace tmux_cxx {
Result<WindowSnapshot> Server::rename_window(WindowId window_id, std::string name) {
    const auto window_entry = state_->windows.find(window_id);
    if (window_entry == state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    if (!valid_window_name(name)) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument, "invalid window name"});
    }
    auto &window_state = window_entry->second;
    if (window_state.name != name) {
        window_state.name = std::move(name);
        window_state.revision = ++state_->revision;
    }
    return window(window_id);
}

RenameWindow::Request RenameWindow::read(protocol::native::NativePayloadReader &reader) {
    return {WindowId{reader.u64()}, reader.string()};
}

Result<RenameWindow::Response> RenameWindow::execute(Server &server, Request request) {
    return server.rename_window(request.window, std::move(request.name));
}

protocol::native::OperationReply RenameWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.window(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
