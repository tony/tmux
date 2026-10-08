#include "connection/native_server_channel.hpp"
#include <tmux_cxx/paste_buffer/operations/delete_paste_buffer.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::delete_paste_buffer(std::string name) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(name);
    auto reply = channel_->invoke_operation(DeletePasteBuffer::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(DeletePasteBuffer::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess delete-buffer reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        DeletePasteBuffer::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
