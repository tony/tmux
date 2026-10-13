#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/resize_pane.hpp>

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<PaneSnapshot> ServerConnection::resize_pane(PaneId pane, PaneResizeDirection direction,
                                                   std::uint32_t cells) {
    if (direction < PaneResizeDirection::left || direction > PaneResizeDirection::down) {
        return std::unexpected(channel_->operation_error(ResizePane::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid pane resize direction"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.u16(std::to_underlying(direction));
    arguments.u32(cells);
    auto reply = channel_->invoke_operation(ResizePane::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto snapshot = reader.pane();
        reader.require_end();
        return snapshot;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ResizePane::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
