#include <tmux_cxx/pane/operations/scroll_copy_mode.hpp>

#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::scroll_copy_mode(PaneId pane_id, CopyScrollMotion motion,
                                      std::uint32_t count) {
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
    auto changed = (*mode)->scroll(motion, count, *keys);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (*changed) {
        pane.touch_copy_display();
    }
    return {};
}

ScrollCopyMode::Request ScrollCopyMode::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}, read_copy_mode_choice(reader, &valid_copy_scroll_motion),
            reader.u32()};
}
Result<ScrollCopyMode::Response> ScrollCopyMode::execute(Server &server, Request request) {
    return server.scroll_copy_mode(request.pane, request.motion, request.count);
}
protocol::native::OperationReply ScrollCopyMode::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
