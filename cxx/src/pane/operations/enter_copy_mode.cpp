#include <tmux_cxx/pane/operations/enter_copy_mode.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::enter_copy_mode(PaneId pane_id) {
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    if (pane.copy_mode) {
        return {};
    }
    auto keys = window_mode_keys(pane.window);
    if (!keys) {
        return std::unexpected(keys.error());
    }
    auto mode = CopyMode::capture(pane.terminal.history_snapshot(), *keys);
    if (!mode) {
        return std::unexpected(mode.error());
    }
    pane.copy_mode = std::move(*mode);
    pane.touch_copy_display();
    return {};
}

EnterCopyMode::Request EnterCopyMode::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<EnterCopyMode::Response> EnterCopyMode::execute(Server &server, Request request) {
    return server.enter_copy_mode(request.pane);
}
protocol::native::OperationReply EnterCopyMode::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
