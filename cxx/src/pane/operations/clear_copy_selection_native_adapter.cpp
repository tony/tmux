#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/clear_copy_selection.hpp>

namespace tmux_cxx {
Result<void> ServerConnection::clear_copy_selection(PaneId pane) {
    if (pane.value == 0) {
        return std::unexpected(channel_->operation_error(ClearCopySelection::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid copy-mode target"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(ClearCopySelection::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(ClearCopySelection::description,
                                                         TmuxErrorCode::protocol,
                                                         "unexpected copy-mode mutation payload"));
    }
    return {};
}
} // namespace tmux_cxx
