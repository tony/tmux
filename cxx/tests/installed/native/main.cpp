#include <tmux_cxx/protocol/native/api_operations.hpp>
#include <tmux_cxx/server/server.hpp>

/** @brief Exercise the installed native target without reading the source tree. */
int main() {
    if (tmux_cxx::protocol::native::api_operations().empty()) {
        return 4;
    }
    tmux_cxx::Server server;
    const auto session = server.create_session("installed", "/bin/cat");
    if (!session || server.sessions().size() != 1) {
        return 1;
    }
    const auto renamed = server.rename_window(session->window, "installed:window");
    if (!renamed || renamed->name != "installed:window" || renamed->pane != session->pane) {
        return 5;
    }
    const auto next = server.next_window(session->id);
    const auto previous = server.previous_window(session->id);
    if (!next || !previous || next->link != session->link || previous->link != session->link) {
        return 6;
    }
    const auto text = server.read_pane_text(session->pane);
    if (!text || text->server_instance != server.server_instance() || text->pane != session->pane ||
        text->rows != 24 || text->columns != 80 || !text->complete) {
        return 2;
    }
    const auto split =
        server.split_pane(session->pane, tmux_cxx::PaneSplitOrientation::left_right, "/bin/cat");
    const auto resized = server.resize_pane(session->pane, tmux_cxx::PaneResizeDirection::right, 5);
    if (!split || !resized || resized->columns != 45 || resized->active) {
        return 7;
    }
    const auto arranged =
        server.select_layout(session->window, tmux_cxx::WindowLayoutPreset::main_vertical_mirrored);
    const auto panes = server.panes(session->window);
    if (!arranged || !panes || panes->size() != 2 || panes->front().id != session->pane ||
        panes->front().left != 2 || panes->back().id != split->id || arranged->pane != split->id) {
        return 8;
    }
    return server.kill_session(session->id) && server.shutdown() ? 0 : 3;
}
