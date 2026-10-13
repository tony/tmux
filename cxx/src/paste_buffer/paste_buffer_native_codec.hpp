#pragma once

#include "protocol/native/framing.hpp"
#include <tmux_cxx/paste_buffer/paste_buffer_snapshot.hpp>

namespace tmux_cxx::protocol::native {
/** @brief Encode one paste-buffer metadata value without its stored bytes. */
void write_paste_buffer_snapshot(NativePayloadWriter &writer, const PasteBufferSnapshot &snapshot);
/** @brief Decode bounded metadata and reject malformed names, sizes, flags, or revisions. */
PasteBufferSnapshot read_paste_buffer_snapshot(NativePayloadReader &reader);
} // namespace tmux_cxx::protocol::native
