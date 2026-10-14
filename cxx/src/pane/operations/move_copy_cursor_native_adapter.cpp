#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/move_copy_cursor.hpp>

namespace tmux_cxx {
Result<void> ServerConnection::move_copy_cursor(PaneId pane, CopyCursorDirection direction,
                                                std::uint32_t count) {
    if (pane.value == 0 || !valid_copy_cursor_direction(direction)) {
        return std::unexpected(channel_->operation_error(MoveCopyCursor::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid copy-mode target or choice"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.u32(std::to_underlying(direction));
    arguments.u32(count);
    auto reply = channel_->invoke_operation(MoveCopyCursor::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(MoveCopyCursor::description,
                                                         TmuxErrorCode::protocol,
                                                         "unexpected copy-mode mutation payload"));
    }
    return {};
}
} // namespace tmux_cxx
