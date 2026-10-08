#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/send_pane_input.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::send_pane_input(PaneId pane, std::string bytes) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.string(bytes);
    auto reply = channel_->invoke_operation(SendPaneInput::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(
            SendPaneInput::description, TmuxErrorCode::protocol, "excess pane input reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        SendPaneInput::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
