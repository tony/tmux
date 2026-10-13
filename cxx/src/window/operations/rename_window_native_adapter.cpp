#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/rename_window.hpp>

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<WindowSnapshot> ServerConnection::rename_window(WindowId window, std::string name) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window.value);
    arguments.string(name);
    auto reply = channel_->invoke_operation(RenameWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto renamed = reader.window();
        reader.require_end();
        return renamed;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(RenameWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        RenameWindow::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
