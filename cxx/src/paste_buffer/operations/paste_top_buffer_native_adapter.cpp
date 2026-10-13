#include "connection/native_server_channel.hpp"
#include <tmux_cxx/paste_buffer/operations/paste_top_buffer.hpp>

namespace tmux_cxx {
Result<void> ServerConnection::paste_top_buffer(PaneId pane) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(PasteTopBuffer::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(PasteTopBuffer::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess default-paste reply bytes"));
    }
    return {};
}
} // namespace tmux_cxx
