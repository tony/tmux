#include <tmux_cxx/pane/operations/read_pane_text.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "text/utf8_encode.hpp"

namespace tmux_cxx {
Result<PaneTextSnapshot> Server::read_pane_text(PaneId pane_id) const {
    const auto pane_entry = state_->panes.find(pane_id);
    if (pane_entry == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    const auto &pane = *pane_entry->second;
    const auto terminal_snapshot = pane.terminal.snapshot();
    PaneTextSnapshot snapshot{state_->server_instance,
                              pane_id,
                              terminal_snapshot.revision,
                              terminal_snapshot.size.rows,
                              terminal_snapshot.size.columns,
                              terminal_snapshot.cursor.row,
                              terminal_snapshot.cursor.column,
                              terminal_snapshot.cursor_visible,
                              terminal_snapshot.alternate_screen,
                              terminal_snapshot.complete,
                              {},
                              static_cast<std::uint32_t>(terminal_snapshot.history_lines),
                              terminal_snapshot.history_gap,
                              pane.pty.get() >= 0 && pane.program_input.accepting_input(),
                              pane.program_input.delivery_pending() ||
                                  !pane.terminal_replies.empty()};
    snapshot.lines.reserve(terminal_snapshot.size.rows);
    for (std::size_t row = 0; row < terminal_snapshot.size.rows; ++row) {
        std::string line;
        for (std::size_t column = 0; column < terminal_snapshot.size.columns; ++column) {
            for (auto scalar :
                 terminal_snapshot.cells[row * terminal_snapshot.size.columns + column].text) {
                text::append_utf8(line, scalar);
            }
        }
        snapshot.lines.push_back(std::move(line));
    }
    return snapshot;
}

ReadPaneText::Request ReadPaneText::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<ReadPaneText::Response> ReadPaneText::execute(Server &server, Request request) {
    return server.read_pane_text(request.pane);
}
protocol::native::OperationReply ReadPaneText::write(Response response) {
    protocol::native::NativePayloadWriter writer;
    writer.string(response.server_instance);
    writer.u64(response.pane.value);
    writer.u64(response.revision);
    writer.u32(response.rows);
    writer.u32(response.columns);
    writer.u32(response.cursor_row);
    writer.u32(response.cursor_column);
    writer.u32(response.history_lines);
    writer.u16((response.cursor_visible ? 1 : 0) | (response.alternate_screen ? 2 : 0) |
               (response.complete ? 4 : 0) | (response.history_gap ? 8 : 0) |
               (response.input_open ? 16 : 0) | (response.input_pending ? 32 : 0));
    for (const auto &line : response.lines) {
        writer.string(line);
    }
    return std::move(writer.bytes);
}
} // namespace tmux_cxx
