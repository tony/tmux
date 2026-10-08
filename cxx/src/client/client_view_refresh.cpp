#include "client/client_view_refresh.hpp"

#include "client/client.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
std::vector<ClientId> ClientViewRefresh::queue_invalidated(Server &server) {
    std::vector<ClientId> failed_clients;
    for (auto &[client_id, client] : server.state_->clients) {
        if (!client->session || !client->terminal || !client->terminal->active() ||
            client->terminal->output_pending()) {
            continue;
        }
        const auto session_entry = server.state_->sessions.find(*client->session);
        if (session_entry == server.state_->sessions.end()) {
            failed_clients.push_back(client_id);
            continue;
        }
        const auto link_entry = server.state_->window_links.find(session_entry->second.current);
        const auto window_entry = link_entry == server.state_->window_links.end()
                                      ? server.state_->windows.end()
                                      : server.state_->windows.find(link_entry->second.window);
        if (window_entry == server.state_->windows.end()) {
            failed_clients.push_back(client_id);
            continue;
        }
        ClientWindowViewSnapshot window_view{
            window_entry->second.id, window_entry->second.revision, window_entry->second.size, {}};
        const auto pane_geometry = window_entry->second.layout.panes();
        window_view.panes.reserve(pane_geometry.size());
        bool complete_layout = true;
        for (const auto &geometry : pane_geometry) {
            const auto pane_entry = server.state_->panes.find(geometry.pane);
            if (pane_entry == server.state_->panes.end()) {
                complete_layout = false;
                break;
            }
            window_view.panes.push_back({geometry, pane_entry->second->terminal.snapshot(),
                                         geometry.pane == window_entry->second.active});
        }
        if (!complete_layout) {
            failed_clients.push_back(client_id);
            continue;
        }
        const auto viewport = client->terminal->size();
        if (!client->view.needs_redraw(window_view, viewport)) {
            continue;
        }
        auto redraw_bytes = client->view.compose_redraw(window_view, viewport);
        if (!redraw_bytes || !client->terminal->queue_output(std::move(*redraw_bytes))) {
            failed_clients.push_back(client_id);
            continue;
        }
        client->view.record_queued_redraw(window_view, viewport);
    }
    return failed_clients;
}
} // namespace tmux_cxx
