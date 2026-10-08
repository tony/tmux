#include "connection/native_server_channel.hpp"
#include <tmux_cxx/paste_buffer/operations/paste_buffer_into_pane.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::paste_buffer_into_pane(std::string name, PaneId pane) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(name);
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(PasteBufferIntoPane::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(PasteBufferIntoPane::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess paste-buffer reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        PasteBufferIntoPane::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
