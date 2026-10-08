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
    const auto text = server.read_pane_text(session->pane);
    if (!text || text->server_instance != server.server_instance() || text->pane != session->pane ||
        text->rows != 24 || text->columns != 80 || !text->complete) {
        return 2;
    }
    return server.kill_session(session->id) && server.shutdown() ? 0 : 3;
}
