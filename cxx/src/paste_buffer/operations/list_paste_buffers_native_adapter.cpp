#include "connection/native_server_channel.hpp"
#include "paste_buffer/paste_buffer.hpp"
#include "paste_buffer/paste_buffer_native_codec.hpp"
#include <tmux_cxx/paste_buffer/operations/list_paste_buffers.hpp>

#include <set>
#include <stdexcept>

namespace tmux_cxx {
Result<std::vector<PasteBufferSnapshot>> ServerConnection::paste_buffers() {
    auto reply = channel_->invoke_operation(ListPasteBuffers::description,
                                            protocol::native::NativePayloadWriter{});
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto count = reader.u32();
        if (count > paste_buffer_named_limit + paste_buffer_automatic_limit) {
            throw std::invalid_argument("paste-buffer list exceeds its entry bound");
        }
        std::vector<PasteBufferSnapshot> snapshots;
        snapshots.reserve(count);
        std::set<std::string_view> names;
        std::size_t automatic_count = 0;
        std::size_t named_count = 0;
        for (std::uint32_t index = 0; index < count; ++index) {
            snapshots.push_back(protocol::native::read_paste_buffer_snapshot(reader));
            const auto &snapshot = snapshots.back();
            if (!names.insert(snapshot.name).second) {
                throw std::invalid_argument("paste-buffer list repeats a name");
            }
            if (snapshot.automatic) {
                ++automatic_count;
            } else {
                ++named_count;
            }
            if (automatic_count > paste_buffer_automatic_limit ||
                named_count > paste_buffer_named_limit ||
                (index != 0 && snapshot.revision >= snapshots[index - 1].revision)) {
                throw std::invalid_argument("paste-buffer list violates class bounds or ordering");
            }
        }
        reader.require_end();
        return snapshots;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(channel_->operation_error(ListPasteBuffers::description,
                                                         TmuxErrorCode::protocol, error.what()));
    } catch (const std::length_error &error) {
        return std::unexpected(channel_->operation_error(ListPasteBuffers::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
