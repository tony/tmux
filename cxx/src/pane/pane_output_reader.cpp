#include "pane/pane_output_reader.hpp"

#include "server/state.hpp"

namespace tmux_cxx {
Result<PaneOutputCursor> PaneOutputReader::tail(const Server &server, PaneId pane) {
    const auto pane_entry = server.state_->panes.find(pane);
    if (pane_entry == server.state_->panes.end()) {
        return std::unexpected(TmuxError{
            TmuxErrorCode::not_found, "pane no longer exists", {}, server.state_->server_instance});
    }
    return PaneOutputCursor{server.state_->server_instance, pane,
                            pane_entry->second->output.tail_offset()};
}

Result<PaneOutputChunk> PaneOutputReader::read(const Server &server,
                                               const PaneOutputCursor &cursor) {
    if (cursor.server_instance != server.state_->server_instance) {
        return std::unexpected(TmuxError{TmuxErrorCode::wrong_server,
                                         "pane output cursor belongs to another server",
                                         {},
                                         server.state_->server_instance});
    }
    const auto pane_entry = server.state_->panes.find(cursor.pane);
    if (pane_entry == server.state_->panes.end()) {
        return std::unexpected(TmuxError{
            TmuxErrorCode::not_found, "pane no longer exists", {}, server.state_->server_instance});
    }
    auto output = pane_entry->second->output.read_from(cursor.offset);
    if (!output) {
        output.error().server_instance = server.state_->server_instance;
        return std::unexpected(output.error());
    }
    return PaneOutputChunk{std::move(output->bytes),
                           {server.state_->server_instance, cursor.pane, output->next_offset}};
}
} // namespace tmux_cxx
