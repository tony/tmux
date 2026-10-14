#include <tmux_cxx/pane/operations/cancel_copy_mode.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::cancel_copy_mode(PaneId pane_id) {
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    if (pane.copy_mode) {
        pane.copy_mode.reset();
        pane.touch_copy_display();
    }
    return {};
}

CancelCopyMode::Request CancelCopyMode::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<CancelCopyMode::Response> CancelCopyMode::execute(Server &server, Request request) {
    return server.cancel_copy_mode(request.pane);
}
protocol::native::OperationReply CancelCopyMode::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
