#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/read_window.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<WindowSnapshot> ServerConnection::window(WindowId window) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window.value);
    auto reply = channel_->invoke_operation(ReadWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto window = reader.window();
        reader.require_end();
        return window;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ReadWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
