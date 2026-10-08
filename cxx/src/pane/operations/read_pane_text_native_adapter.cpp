#include "connection/native_server_channel.hpp"
#include "text/utf8.hpp"
#include <tmux_cxx/pane/operations/read_pane_text.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<PaneTextSnapshot> ServerConnection::read_pane_text(PaneId pane) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(ReadPaneText::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        PaneTextSnapshot snapshot{};
        snapshot.server_instance = reader.string();
        snapshot.pane = PaneId{reader.u64()};
        snapshot.revision = reader.u64();
        snapshot.rows = reader.u32();
        snapshot.columns = reader.u32();
        snapshot.cursor_row = reader.u32();
        snapshot.cursor_column = reader.u32();
        snapshot.history_lines = reader.u32();
        const auto flags = reader.u16();
        const auto server_instance = channel_->observed_server_instance();
        if (snapshot.rows == 0 || snapshot.columns == 0 || snapshot.rows > 512 ||
            snapshot.columns > 512 || snapshot.rows * snapshot.columns > 16384 ||
            snapshot.cursor_row >= snapshot.rows || snapshot.cursor_column >= snapshot.columns ||
            snapshot.history_lines > 1000 || flags > 63 || snapshot.pane != pane ||
            snapshot.server_instance != server_instance) {
            throw std::invalid_argument("invalid pane text ownership or terminal dimensions");
        }
        snapshot.cursor_visible = flags & 1;
        snapshot.alternate_screen = flags & 2;
        snapshot.complete = flags & 4;
        snapshot.history_gap = flags & 8;
        snapshot.input_open = flags & 16;
        snapshot.input_pending = flags & 32;
        for (std::size_t row = 0; row < snapshot.rows; ++row) {
            auto line = reader.string();
            if (line.size() > 24 * snapshot.columns || !text::valid_utf8(line)) {
                throw std::invalid_argument("invalid UTF-8 or excessive pane text row");
            }
            snapshot.lines.push_back(std::move(line));
        }
        reader.require_end();
        return snapshot;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ReadPaneText::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
