#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window_link/operations/unlink_window.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::unlink_window(WindowLinkId window_link) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window_link.value);
    auto reply = channel_->invoke_operation(UnlinkWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(UnlinkWindow::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess window unlink reply bytes"));
    }
    return {};
}
} // namespace tmux_cxx
