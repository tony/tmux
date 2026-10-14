#pragma once

#include <tmux_cxx/pane/copy_mode_snapshot.hpp>

namespace tmux_cxx::protocol::native {
class NativePayloadReader;
class NativePayloadWriter;
/** @brief Encode copied mode state without exposing frozen backing storage. */
void write_copy_mode_snapshot(NativePayloadWriter &writer, const CopyModeSnapshot &snapshot);
/** @brief Decode bounded frozen state and reject malformed coordinates, tags, or text. */
CopyModeSnapshot read_copy_mode_snapshot(NativePayloadReader &reader);
} // namespace tmux_cxx::protocol::native
