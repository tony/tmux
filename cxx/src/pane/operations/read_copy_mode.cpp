#include <tmux_cxx/pane/operations/read_copy_mode.hpp>

#include "pane/copy_mode_native_codec.hpp"
#include "pane/copy_mode_target.hpp"
#include "server/state.hpp"
#include "text/utf8_encode.hpp"

namespace tmux_cxx {
Result<CopyModeSnapshot> Server::copy_mode(PaneId pane_id) const {
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    CopyModeSnapshot snapshot{state_->server_instance, pane.id, pane.display_revision(),
                              std::nullopt};
    if (!pane.copy_mode) {
        return snapshot;
    }
    auto keys = window_mode_keys(pane.window);
    if (!keys) {
        return std::unexpected(keys.error());
    }
    const auto &mode = pane.copy_mode->state();
    const auto screen = pane.screen_snapshot();
    CopyModeStateSnapshot observed{mode.size.rows,        mode.size.columns,
                                   mode.retained_rows,    mode.viewport_top,
                                   mode.cursor,           std::nullopt,
                                   mode.source_revision,  mode.source_history_lines,
                                   mode.alternate_screen, mode.complete,
                                   mode.history_gap,      *keys,
                                   mode.highlight_keys,   {}};
    if (mode.selection_anchor) {
        observed.selection = CopySelectionSnapshot{*mode.selection_anchor, mode.cursor};
    }
    observed.lines.reserve(mode.size.rows);
    for (std::uint32_t row = 0; row < mode.size.rows; ++row) {
        std::string line;
        for (std::uint32_t column = 0; column < mode.size.columns; ++column) {
            const auto &cell = screen.cells[row * mode.size.columns + column];
            if (cell.width == 0) {
                continue;
            }
            for (const auto scalar : cell.text) {
                text::append_utf8(line, scalar);
            }
        }
        observed.lines.push_back(std::move(line));
    }
    snapshot.mode = std::move(observed);
    return snapshot;
}
ReadCopyMode::Request ReadCopyMode::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}
Result<ReadCopyMode::Response> ReadCopyMode::execute(Server &server, Request request) {
    return server.copy_mode(request.pane);
}
protocol::native::OperationReply ReadCopyMode::write(Response response) {
    protocol::native::NativePayloadWriter writer;
    protocol::native::write_copy_mode_snapshot(writer, response);
    return std::move(writer.bytes);
}
} // namespace tmux_cxx
