#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window_link/operations/previous_window.hpp>

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<SessionSnapshot> ServerConnection::previous_window(SessionId session) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(session.value);
    auto reply = channel_->invoke_operation(PreviousWindow::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto selected = reader.session();
        reader.require_end();
        return selected;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(PreviousWindow::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
