#include "connection/native_server_channel.hpp"
#include <tmux_cxx/paste_buffer/operations/read_paste_buffer.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::string> ServerConnection::read_paste_buffer(std::string name) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.string(name);
    auto reply = channel_->invoke_operation(ReadPasteBuffer::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    protocol::native::NativePayloadReader reader(*reply);
    auto bytes = reader.string();
    reader.require_end();
    return bytes;
} catch (const std::invalid_argument &error) {
    return std::unexpected(channel_->operation_error(ReadPasteBuffer::description,
                                                     TmuxErrorCode::protocol, error.what()));
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        ReadPasteBuffer::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
