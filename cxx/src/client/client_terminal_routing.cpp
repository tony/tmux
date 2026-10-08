#include "client/client_terminal_routing.hpp"

#include "client/client.hpp"
#include "server/state.hpp"

#include <poll.h>

namespace tmux_cxx {
namespace {
constexpr short route_failure = POLLERR | POLLHUP | POLLNVAL; ///< Terminal route closure events.
}

std::vector<ClientTerminalRoute> ClientTerminalRouting::capture(const Server &server) {
    std::vector<ClientTerminalRoute> routes;
    for (const auto &[client_id, client] : server.state_->clients) {
        if (client->session && client->terminal && client->terminal->active()) {
            routes.push_back({client_id, client->terminal->input_descriptor(),
                              client->terminal->output_descriptor(),
                              client->key_input.read_capacity() != 0,
                              client->terminal->output_pending()});
        }
    }
    return routes;
}

ClientTerminalRouteOutcome
ClientTerminalRouting::route_ready_io(Server &server, ClientId client_id,
                                      const key_table::KeyTableCatalog &key_tables,
                                      short input_events, short output_events) {
    const auto client_entry = server.state_->clients.find(client_id);
    if (client_entry == server.state_->clients.end() || !client_entry->second->session ||
        !client_entry->second->terminal || !client_entry->second->terminal->active()) {
        return ClientTerminalRouteOutcome::superseded;
    }
    if ((input_events & route_failure) != 0 || (output_events & route_failure) != 0) {
        return ClientTerminalRouteOutcome::failed;
    }
    auto &client = *client_entry->second;
    if ((input_events & POLLIN) != 0) {
        auto terminal_input = client.terminal->read_input(client.key_input.read_capacity());
        if (!terminal_input || terminal_input->closed) {
            return ClientTerminalRouteOutcome::failed;
        }
        if (auto appended = client.key_input.append(terminal_input->bytes); !appended) {
            return ClientTerminalRouteOutcome::failed;
        }
    }
    while (true) {
        if (!client.session) {
            return ClientTerminalRouteOutcome::routed;
        }
        const auto session_entry = server.state_->sessions.find(*client.session);
        if (session_entry == server.state_->sessions.end()) {
            return ClientTerminalRouteOutcome::failed;
        }
        const auto link_entry = server.state_->window_links.find(session_entry->second.current);
        if (link_entry == server.state_->window_links.end()) {
            return ClientTerminalRouteOutcome::failed;
        }
        const auto window_entry = server.state_->windows.find(link_entry->second.window);
        if (window_entry == server.state_->windows.end()) {
            return ClientTerminalRouteOutcome::failed;
        }
        const auto pane_entry = server.state_->panes.find(window_entry->second.active);
        if (pane_entry == server.state_->panes.end()) {
            return ClientTerminalRouteOutcome::failed;
        }
        auto prefix_keys = server.session_prefix_keys(session_entry->first);
        if (!prefix_keys) {
            return ClientTerminalRouteOutcome::failed;
        }
        auto key_input = client.key_input.route_pending(
            server, client_id, window_entry->first, pane_entry->first, *prefix_keys,
            pane_entry->second->client_program_input_capacity(), key_tables);
        if (!key_input) {
            return ClientTerminalRouteOutcome::failed;
        }
        if (*key_input == key_table::KeyBindingEffect::client_detached) {
            return ClientTerminalRouteOutcome::routed;
        }
        if (*key_input != key_table::KeyBindingEffect::selected_pane_changed) {
            break;
        }
    }
    if ((output_events & POLLOUT) != 0) {
        std::size_t remaining_output_budget = 65536;
        if (!client.terminal->flush_output(remaining_output_budget)) {
            return ClientTerminalRouteOutcome::failed;
        }
    }
    return ClientTerminalRouteOutcome::routed;
}
} // namespace tmux_cxx
