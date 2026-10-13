#include "paste_buffer/paste_buffer_native_codec.hpp"
#include "paste_buffer/paste_buffer.hpp"

#include <stdexcept>

namespace tmux_cxx::protocol::native {
void write_paste_buffer_snapshot(NativePayloadWriter &writer, const PasteBufferSnapshot &snapshot) {
    writer.string(snapshot.name);
    writer.u32(snapshot.byte_count);
    writer.u16(snapshot.automatic ? 1 : 0);
    writer.u64(snapshot.revision);
}

PasteBufferSnapshot read_paste_buffer_snapshot(NativePayloadReader &reader) {
    auto name = reader.string();
    const auto byte_count = reader.u32();
    const auto automatic = reader.u16();
    const auto revision = reader.u64();
    if (!valid_paste_buffer_name(name) || byte_count == 0 || byte_count > paste_buffer_byte_limit ||
        automatic > 1 || revision == 0) {
        throw std::invalid_argument("invalid paste-buffer metadata");
    }
    return {std::move(name), byte_count, automatic != 0, revision};
}
} // namespace tmux_cxx::protocol::native
