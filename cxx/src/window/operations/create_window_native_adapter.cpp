#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/create_window.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<WindowLinkSnapshot> ServerConnection::create_window(SessionId session, std::uint32_t index,
                                                           std::string name, std::string command) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(session.value);
    arguments.u32(index);
    arguments.string(name);
    arguments.string(command);
    auto reply = channel_->invoke_operation(CreateWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto link = reader.window_link();
        reader.require_end();
        return link;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(CreateWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
