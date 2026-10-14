#include <tmux_cxx/pane/operations/clear_copy_selection.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::clear_copy_selection(PaneId pane_id) {
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
    auto changed = (*mode)->clear_selection(*keys);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (*changed) {
        pane.touch_copy_display();
    }
    return {};
}

ClearCopySelection::Request
ClearCopySelection::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<ClearCopySelection::Response> ClearCopySelection::execute(Server &server, Request request) {
    return server.clear_copy_selection(request.pane);
}
protocol::native::OperationReply ClearCopySelection::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
