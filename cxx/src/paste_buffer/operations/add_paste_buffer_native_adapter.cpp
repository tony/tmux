#include "connection/native_server_channel.hpp"
#include "paste_buffer/paste_buffer_native_codec.hpp"
#include <tmux_cxx/paste_buffer/operations/add_paste_buffer.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::optional<PasteBufferSnapshot>>
ServerConnection::add_paste_buffer(std::string bytes) try {
    protocol::native::NativePayloadWriter arguments;
    const auto byte_count = bytes.size();
    arguments.string(bytes);
    auto reply = channel_->invoke_operation(AddPasteBuffer::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto present = reader.u16();
        if (present > 1) {
            throw std::invalid_argument("invalid automatic buffer optional flag");
        }
        std::optional<PasteBufferSnapshot> snapshot;
        if (present != 0) {
            snapshot = protocol::native::read_paste_buffer_snapshot(reader);
        }
        reader.require_end();
        if (snapshot.has_value() != (byte_count != 0) ||
            (snapshot && (!snapshot->automatic || snapshot->byte_count != byte_count))) {
            throw std::invalid_argument("automatic buffer reply contradicts requested bytes");
        }
        return snapshot;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(channel_->operation_error(AddPasteBuffer::description,
                                                         TmuxErrorCode::protocol, error.what()));
    } catch (const std::length_error &error) {
        return std::unexpected(channel_->operation_error(AddPasteBuffer::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        AddPasteBuffer::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
