#include <tmux_cxx/pane/operations/move_copy_cursor_to.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::move_copy_cursor_to(PaneId pane_id, CopyCursorDestination destination) {
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    auto mode = active_copy_mode(pane);
    if (!mode) {
        return std::unexpected(mode.error());
    }
    auto keys = window_mode_keys(pane.window);
    if (!keys) {
        return std::unexpected(keys.error());
    }
    auto changed = (*mode)->move_to(destination, *keys);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (*changed) {
        pane.touch_copy_display();
    }
    return {};
}

MoveCopyCursorTo::Request MoveCopyCursorTo::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}, read_copy_mode_choice(reader, &valid_copy_cursor_destination)};
}
Result<MoveCopyCursorTo::Response> MoveCopyCursorTo::execute(Server &server, Request request) {
    return server.move_copy_cursor_to(request.pane, request.destination);
}
protocol::native::OperationReply MoveCopyCursorTo::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
