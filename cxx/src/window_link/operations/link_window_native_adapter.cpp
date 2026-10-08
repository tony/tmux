#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window_link/operations/link_window.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<WindowLinkSnapshot> ServerConnection::link_window(WindowId window_id, SessionId session,
                                                         std::uint32_t index) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window_id.value);
    arguments.u64(session.value);
    arguments.u32(index);
    auto reply = channel_->invoke_operation(LinkWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto link = reader.window_link();
        reader.require_end();
        return link;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(LinkWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
