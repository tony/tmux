#include "window/window_size.hpp"

#include "server/state.hpp"

namespace tmux_cxx {
Result<void> LatestClientWindowSize::apply(Server &server, SessionId session_id,
                                           terminal::CellSize size) {
    const auto session = server.state_->sessions.find(session_id);
    if (session == server.state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    const auto link = server.state_->window_links.find(session->second.current);
    const auto window = link == server.state_->window_links.end()
                            ? server.state_->windows.end()
                            : server.state_->windows.find(link->second.window);
    if (window == server.state_->windows.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "selected window no longer exists"});
    }
    return apply_to_window(server, window->first, size);
}

Result<void> LatestClientWindowSize::apply_to_window(Server &server, WindowId window_id,
                                                     terminal::CellSize size) {
    const auto window = server.state_->windows.find(window_id);
    if (window == server.state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    auto candidate_layout = window->second.layout;
    const auto candidate_size = candidate_layout.resize(size);
    if (!candidate_size) {
        return std::unexpected(candidate_size.error());
    }
    if (window->second.size == *candidate_size) {
        return {};
    }
    const auto previous_geometry = window->second.layout.panes();
    const auto candidate_geometry = candidate_layout.panes();
    if (auto resized = server.state_->resize_pane_terminals(candidate_geometry, previous_geometry);
        !resized) {
        return resized;
    }
    window->second.layout = std::move(candidate_layout);
    window->second.size = *candidate_size;
    window->second.revision = ++server.state_->revision;
    for (auto &[client_id, client] : server.state_->clients) {
        (void)client_id;
        if (!client->session) {
            continue;
        }
        const auto client_session = server.state_->sessions.find(*client->session);
        if (client_session == server.state_->sessions.end()) {
            continue;
        }
        const auto client_link = server.state_->window_links.find(client_session->second.current);
        if (client_link != server.state_->window_links.end() &&
            client_link->second.window == window->second.id) {
            client->view.invalidate();
        }
    }
    return {};
}
} // namespace tmux_cxx
