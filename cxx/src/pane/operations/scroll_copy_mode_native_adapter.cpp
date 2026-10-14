#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/scroll_copy_mode.hpp>

namespace tmux_cxx {
Result<void> ServerConnection::scroll_copy_mode(PaneId pane, CopyScrollMotion motion,
                                                std::uint32_t count) {
    if (pane.value == 0 || !valid_copy_scroll_motion(motion)) {
        return std::unexpected(channel_->operation_error(ScrollCopyMode::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid copy-mode target or choice"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.u32(std::to_underlying(motion));
    arguments.u32(count);
    auto reply = channel_->invoke_operation(ScrollCopyMode::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(ScrollCopyMode::description,
                                                         TmuxErrorCode::protocol,
                                                         "unexpected copy-mode mutation payload"));
    }
    return {};
}
} // namespace tmux_cxx
