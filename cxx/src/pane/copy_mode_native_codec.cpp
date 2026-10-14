#include "pane/copy_mode_native_codec.hpp"

#include "protocol/native/framing.hpp"
#include "text/utf8.hpp"

#include <stdexcept>
#include <utility>

namespace tmux_cxx::protocol::native {
namespace {
constexpr std::uint32_t maximum_backing_lines = 1000U * 512U + 16384U + 512U;
///< Maximum captured cells plus reflow padding, independent of the original history-row count.

/** @brief Reject copied mode facts outside the bounded viewport and frozen backing. */
void validate_copy_mode_state(const CopyModeStateSnapshot &mode) {
    if (mode.rows == 0 || mode.columns == 0 || mode.rows > 512 || mode.columns > 512 ||
        mode.rows * mode.columns > 16384 || mode.backing_lines < mode.rows ||
        mode.backing_lines > maximum_backing_lines ||
        mode.viewport_top > mode.backing_lines - mode.rows || mode.source_history_lines > 1000 ||
        !valid_window_mode_keys(mode.mode_keys) || !valid_window_mode_keys(mode.highlight_keys) ||
        (mode.alternate_screen && (mode.source_history_lines != 0 || mode.history_gap))) {
        throw std::invalid_argument("invalid copy-mode dimensions, keys, or source history");
    }
    /** @brief Retain legal one-past-end columns without accepting cells outside frozen backing. */
    const auto valid_position = [&mode](CopyPosition position) {
        return position.row < mode.backing_lines && position.column <= mode.columns;
    };
    if (!valid_position(mode.cursor) || mode.cursor.row < mode.viewport_top ||
        mode.cursor.row >= mode.viewport_top + mode.rows ||
        (mode.selection &&
         (!valid_position(mode.selection->anchor) || !valid_position(mode.selection->end) ||
          mode.selection->end != mode.cursor))) {
        throw std::invalid_argument("invalid copy-mode cursor or selection coordinates");
    }
}

/** @brief Decode one backing position without narrowing either coordinate. */
CopyPosition read_copy_position(NativePayloadReader &reader) {
    return {reader.u32(), reader.u32()};
}

/** @brief Encode one backing position without conflating it with terminal-screen coordinates. */
void write_copy_position(NativePayloadWriter &writer, CopyPosition position) {
    writer.u32(position.row);
    writer.u32(position.column);
}
} // namespace

void write_copy_mode_snapshot(NativePayloadWriter &writer, const CopyModeSnapshot &snapshot) {
    writer.string(snapshot.server_instance);
    writer.u64(snapshot.pane.value);
    writer.u64(snapshot.display_revision);
    writer.u16(snapshot.mode.has_value() ? 1 : 0);
    if (!snapshot.mode) {
        return;
    }
    const auto &mode = *snapshot.mode;
    validate_copy_mode_state(mode);
    if (mode.lines.size() != mode.rows || snapshot.display_revision == 0) {
        throw std::invalid_argument("invalid copy-mode visible rows or display revision");
    }
    writer.u32(mode.rows);
    writer.u32(mode.columns);
    writer.u32(mode.backing_lines);
    writer.u32(mode.viewport_top);
    write_copy_position(writer, mode.cursor);
    writer.u64(mode.source_revision);
    writer.u32(mode.source_history_lines);
    writer.u16((mode.alternate_screen ? 1 : 0) | (mode.complete ? 2 : 0) |
               (mode.history_gap ? 4 : 0));
    writer.u32(std::to_underlying(mode.mode_keys));
    writer.u32(std::to_underlying(mode.highlight_keys));
    writer.u16(mode.selection.has_value() ? 1 : 0);
    if (mode.selection) {
        write_copy_position(writer, mode.selection->anchor);
        write_copy_position(writer, mode.selection->end);
    }
    for (const auto &line : mode.lines) {
        writer.string(line);
    }
}

CopyModeSnapshot read_copy_mode_snapshot(NativePayloadReader &reader) {
    CopyModeSnapshot snapshot{};
    snapshot.server_instance = reader.string();
    snapshot.pane = PaneId{reader.u64()};
    snapshot.display_revision = reader.u64();
    const auto present = reader.u16();
    if (present > 1 || snapshot.pane.value == 0) {
        throw std::invalid_argument("invalid copy-mode pane or optional flag");
    }
    if (present == 0) {
        return snapshot;
    }
    CopyModeStateSnapshot mode{};
    mode.rows = reader.u32();
    mode.columns = reader.u32();
    mode.backing_lines = reader.u32();
    mode.viewport_top = reader.u32();
    mode.cursor = read_copy_position(reader);
    mode.source_revision = reader.u64();
    mode.source_history_lines = reader.u32();
    const auto flags = reader.u16();
    mode.mode_keys = static_cast<WindowModeKeys>(reader.u32());
    mode.highlight_keys = static_cast<WindowModeKeys>(reader.u32());
    const auto selection_present = reader.u16();
    if (flags > 7 || selection_present > 1 || snapshot.display_revision == 0) {
        throw std::invalid_argument("invalid copy-mode flags or active display revision");
    }
    mode.alternate_screen = flags & 1;
    mode.complete = flags & 2;
    mode.history_gap = flags & 4;
    if (selection_present != 0) {
        mode.selection =
            CopySelectionSnapshot{read_copy_position(reader), read_copy_position(reader)};
    }
    validate_copy_mode_state(mode);
    mode.lines.reserve(mode.rows);
    for (std::uint32_t row = 0; row < mode.rows; ++row) {
        auto line = reader.string();
        if (line.size() > 24U * mode.columns || !text::valid_utf8(line)) {
            throw std::invalid_argument("invalid copy-mode UTF-8 visible row");
        }
        mode.lines.push_back(std::move(line));
    }
    snapshot.mode = std::move(mode);
    return snapshot;
}
} // namespace tmux_cxx::protocol::native
