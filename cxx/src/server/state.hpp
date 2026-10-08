#pragma once

#include "client/client.hpp"
#include "pane/pane.hpp"
#include "pane/program.hpp"
#include "paste_buffer/paste_buffer.hpp"
#include "session/session.hpp"
#include "session/session_event_journal.hpp"
#include "session/session_options.hpp"
#include "window/window.hpp"
#include "window_link/window_link.hpp"
#include <tmux_cxx/server/server.hpp>

#include <map>
#include <vector>

namespace tmux_cxx {
/** @brief Own the server's entity graph and allocate identities that are never reused during its
 * lifetime. */
struct Server::State {
    std::string server_instance;             ///< Random identity scoped to this server lifetime.
    std::uint64_t next_session_identity = 1; ///< Session identity allocation counter.
    std::uint64_t next_window_identity = 1;  ///< Window identity allocation counter.
    std::uint64_t next_window_link_identity = 1; ///< Window-link identity allocation counter.
    std::uint64_t next_pane_identity = 1;        ///< Pane identity allocation counter.
    std::uint64_t next_client_identity = 1;      ///< Client identity allocation counter.
    std::uint64_t revision = 0; ///< Server mutation sequence shared by entity snapshots.
    options::OptionTable global_session_options =
        session_options::global_defaults(); ///< Defaults and global values inherited by sessions.
    std::map<SessionId, Session>
        sessions;                       ///< Persistent session entities indexed by stable identity.
    std::map<WindowId, Window> windows; ///< Shared windows independent of session membership.
    std::map<WindowLinkId, WindowLink>
        window_links; ///< Session memberships retaining shared windows.
    std::map<PaneId, std::unique_ptr<Pane>>
        panes; ///< Owned PTYs and retained pane output indexed by identity.
    std::map<PaneId, PaneProgram>
        pane_programs; ///< Owned direct children retained through termination and reaping.
    std::map<ClientId, std::unique_ptr<Client>>
        clients; ///< Connection-lifetime clients independent of persistent session ownership.
    std::vector<PasteBuffer>
        paste_buffers; ///< Server-global named buffers in most-recently-stored order.
    SessionEventJournal
        session_event_journal; ///< Bounded committed facts for owner-qualified observers.
    bool shutdown_attempted =
        false; ///< Prevent another admission or repeated blocking shutdown attempt.
    /** @brief Materialize the session's selected link, shared window, and active pane. */
    SessionSnapshot session_snapshot(const Session &session) const;
    /** @brief Materialize a session membership without exposing server entity pointers. */
    WindowLinkSnapshot window_link_snapshot(const WindowLink &link) const;
    /** @brief Materialize attachment and terminal dimensions without exposing a live client. */
    ClientSnapshot client_snapshot(const Client &client) const;
    /** @brief Materialize one layout leaf with its owning window revision and selection state. */
    PaneSnapshot pane_snapshot(const Window &window, const PaneGeometry &geometry) const;
    /** @brief Find one named paste buffer without changing its most-recent ordering. */
    std::vector<PasteBuffer>::iterator find_paste_buffer(std::string_view name);
    /** @brief Find one named paste buffer through a read-only server state. */
    std::vector<PasteBuffer>::const_iterator find_paste_buffer(std::string_view name) const;
    /** @brief Release one membership and destroy its window only after the final membership
     * disappears. */
    void remove_window_link(WindowLinkId window_link_id);
    /** @brief Close a removed pane's PTY while retaining its child until nonblocking reaping
     * completes. */
    void retire_pane(PaneId pane_id);
};
} // namespace tmux_cxx
