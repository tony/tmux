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
    for (const auto &geometry : candidate_geometry) {
        const auto pane = server.state_->panes.find(geometry.pane);
        if (pane == server.state_->panes.end()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::not_found, "layout pane no longer exists"});
        }
        if (!pane->second->terminal.snapshot().complete) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "pane terminal state is incomplete"});
        }
    }
    /** Restore the former pane geometries if a later terminal refuses the candidate layout. */
    const auto restore_previous_geometry = [&server, &previous_geometry]() -> Result<void> {
        for (const auto &geometry : previous_geometry) {
            const auto pane = server.state_->panes.find(geometry.pane);
            if (pane != server.state_->panes.end()) {
                if (auto restored = pane->second->resize_terminal(geometry.size); !restored) {
                    return std::unexpected(TmuxError{
                        restored.error().code,
                        "window resize failed and prior pane geometry could not be restored"});
                }
            }
        }
        return {};
    };
    for (const auto &geometry : candidate_geometry) {
        const auto pane = server.state_->panes.find(geometry.pane);
        if (auto resized = pane->second->resize_terminal(geometry.size); !resized) {
            if (auto restored = restore_previous_geometry(); !restored) {
                return std::unexpected(restored.error());
            }
            return std::unexpected(resized.error());
        }
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
