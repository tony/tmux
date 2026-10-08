#include "protocol/tmux/control/selected_session_output_feed.hpp"

#include "pane/pane_output_reader.hpp"
#include "protocol/tmux/control/limits.hpp"
#include "protocol/tmux/control/pane_output.hpp"
#include "protocol/tmux/control/session_changed.hpp"
#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <set>

namespace tmux_cxx::protocol::tmux::control {
namespace {
/** @brief Copy the selected session or report that the client relationship became stale. */
Result<SessionSnapshot> selected_session_snapshot(Server &server, SessionId session) {
    auto sessions = server.sessions();
    const auto selected = std::ranges::find(sessions, session, &SessionSnapshot::id);
    if (selected == sessions.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "control client's session no longer exists"});
    }
    return *selected;
}

/** @brief List every live pane linked into one selected session in identity order. */
Result<std::vector<PaneId>> selected_session_panes(Server &server, SessionId session) {
    auto window_links = server.window_links(session);
    if (!window_links) {
        return std::unexpected(window_links.error());
    }
    std::set<PaneId> pane_identities;
    for (const auto &link : *window_links) {
        auto window_panes = server.panes(link.window);
        if (!window_panes) {
            return std::unexpected(window_panes.error());
        }
        for (const auto &pane : *window_panes) {
            pane_identities.insert(pane.id);
        }
    }
    return std::vector<PaneId>(pane_identities.begin(), pane_identities.end());
}
} // namespace

Result<void> SelectedSessionOutputFeed::queue_new_output(Server &server,
                                                         const ClientSnapshot &client,
                                                         Channel &channel) {
    if (!client.session) {
        selected_session_.reset();
        pane_cursors_.clear();
        return {};
    }
    auto selected_panes = selected_session_panes(server, *client.session);
    if (!selected_panes) {
        return std::unexpected(selected_panes.error());
    }
    if (selected_session_ != client.session) {
        auto selected_session = selected_session_snapshot(server, *client.session);
        if (!selected_session) {
            return std::unexpected(selected_session.error());
        }
        std::map<PaneId, PaneOutputCursor> initial_cursors;
        for (const auto pane : *selected_panes) {
            auto pane_cursor = PaneOutputReader::tail(server, pane);
            if (!pane_cursor) {
                return std::unexpected(pane_cursor.error());
            }
            initial_cursors.emplace(pane, std::move(*pane_cursor));
        }
        if (auto session_change_queued =
                channel.queue_record(format_session_changed(*selected_session));
            !session_change_queued) {
            return session_change_queued;
        }
        selected_session_ = client.session;
        pane_cursors_ = std::move(initial_cursors);
        return {};
    }

    const std::set<PaneId> live_panes(selected_panes->begin(), selected_panes->end());
    std::size_t remaining_raw_bytes = pane_output_bytes_per_turn;
    for (const auto pane : *selected_panes) {
        auto pane_cursor = pane_cursors_.find(pane);
        if (pane_cursor == pane_cursors_.end()) {
            auto pane_tail = PaneOutputReader::tail(server, pane);
            if (!pane_tail) {
                return std::unexpected(pane_tail.error());
            }
            pane_cursors_.emplace(pane, std::move(*pane_tail));
            continue;
        }
        if (remaining_raw_bytes == 0) {
            break;
        }
        auto pane_output = PaneOutputReader::read(server, pane_cursor->second);
        if (!pane_output) {
            return std::unexpected(pane_output.error());
        }
        const auto queued_bytes = std::min(remaining_raw_bytes, pane_output->bytes.size());
        if (queued_bytes == 0) {
            continue;
        }
        if (auto pane_output_queued = channel.queue_record(format_pane_output(
                pane, std::string_view(pane_output->bytes).substr(0, queued_bytes)));
            !pane_output_queued) {
            return pane_output_queued;
        }
        pane_cursor->second.offset += queued_bytes;
        remaining_raw_bytes -= queued_bytes;
    }
    std::erase_if(pane_cursors_,
                  /** @brief Remove cursors for panes no longer linked into the selected session. */
                  [&](const auto &entry) { return !live_panes.contains(entry.first); });
    return {};
}
} // namespace tmux_cxx::protocol::tmux::control
