#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window_link/operations/select_window.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<SessionSnapshot> ServerConnection::select_window(WindowLinkId window_link) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window_link.value);
    auto reply = channel_->invoke_operation(SelectWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto session = reader.session();
        reader.require_end();
        return session;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(SelectWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
