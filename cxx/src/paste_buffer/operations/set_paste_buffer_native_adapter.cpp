#include "connection/native_server_channel.hpp"
#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::set_paste_buffer(std::string name, std::string bytes) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(name);
    arguments.string(bytes);
    auto reply = channel_->invoke_operation(SetPasteBuffer::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(
            SetPasteBuffer::description, TmuxErrorCode::protocol, "excess set-buffer reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        SetPasteBuffer::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
