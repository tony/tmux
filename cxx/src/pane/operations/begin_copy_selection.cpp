#include <tmux_cxx/pane/operations/begin_copy_selection.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::begin_copy_selection(PaneId pane_id) {
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    auto mode = active_copy_mode(pane);
    if (!mode) {
        return std::unexpected(mode.error());
    }
    if ((*mode)->begin_selection()) {
        pane.touch_copy_display();
    }
    return {};
}

BeginCopySelection::Request
BeginCopySelection::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<BeginCopySelection::Response> BeginCopySelection::execute(Server &server, Request request) {
    return server.begin_copy_selection(request.pane);
}
protocol::native::OperationReply BeginCopySelection::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
