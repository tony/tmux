#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/pane_text_snapshot.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/session/session_prefix_keys.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>
#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

#include <chrono>
#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace tmux_cxx {
class StockClientLifecycle;
class ClientTerminalRouting;
class ClientViewRefresh;
class LatestClientWindowSize;
class PaneOutputReader;
class SessionEventReader;
/** @brief Own persistent sessions and PTYs; mutation belongs to the server thread.
 * Callers must leave child reaping and owned process-group signaling to this server. */
class Server {
  public:
    /** @brief Establish an unpredictable identity for this server lifetime. */
    Server();
    /** @brief Stop owned programs with a bounded shutdown; never destroy sessions through
     * connection disposal. */
    ~Server();
    /** @brief Prevent copying the owner of persistent tmux state. */
    Server(const Server &) = delete;
    /** @brief Prevent replacing persistent state through copy assignment. */
    Server &operator=(const Server &) = delete;
    /** @brief Borrow the server identity until server destruction. */
    const std::string &server_instance() const;
    /** @brief Create a persistent session with a real PTY; reject invalid or duplicate names before
     * spawning.
     *
     * Names require UTF-8, at most 128 bytes, and no NUL, colon, period, or line-break characters.
     */
    Result<SessionSnapshot> create_session(std::string name, std::string command);
    /** @brief Materialize sessions in identity order. */
    std::vector<SessionSnapshot> sessions() const;
    /** @brief Rename a live session after validating uniqueness and the replacement name.
     *
     * @see tmux_cxx::Server::create_session for shared name constraints. */
    Result<void> rename_session(SessionId session, std::string name);
    /** @brief Set one implemented server-global session option from canonical tmux text. */
    Result<void> set_global_session_option(std::string_view name, std::string_view value);
    /** @brief Set one live session's local option override from canonical tmux text. */
    Result<void> set_session_option(SessionId session, std::string_view name,
                                    std::string_view value);
    /** @brief Resolve one live session's primary and secondary prefix keys. */
    Result<SessionPrefixKeys> session_prefix_keys(SessionId session) const;
    /** @brief Remove a session's memberships; preserve windows linked into other sessions. */
    Result<void> kill_session(SessionId session);
    /** @brief Materialize connection-lifetime tmux clients without exposing terminal descriptors.
     */
    std::vector<ClientSnapshot> clients() const;
    /** @brief Copy one stock client's selected wire profile, evidence, support, and limits. */
    Result<protocol::tmux::StockConnectionCompatibility>
    stock_client_compatibility(ClientId client) const;
    /** @brief Attach an identified terminal or control client to a retained session. */
    Result<ClientSnapshot> attach_client(ClientId client, SessionId session);
    /** @brief Restore an attached client's terminal and retain both client and session entities. */
    Result<ClientSnapshot> detach_client(ClientId client);
    /** @brief Remeasure an attached client's TTY and apply the latest-client window-size policy. */
    Result<ClientSnapshot> resize_client(ClientId client);
    /** @brief Link a shared window at an unused session index without changing the current
     * selection. */
    Result<WindowLinkSnapshot> link_window(WindowId window, SessionId session, std::uint32_t index);
    /** @brief Remove one window membership; removing the final membership also removes its window
     * and panes. */
    Result<void> unlink_window(WindowLinkId window_link);
    /** @brief Materialize session window memberships in index order. */
    Result<std::vector<WindowLinkSnapshot>> window_links(SessionId session) const;
    /** @brief Materialize a shared window's name, selected pane, and mutation revision. */
    Result<WindowSnapshot> window(WindowId window) const;
    /** @brief Split one pane, start its owned program, and select the materialized new pane. */
    Result<PaneSnapshot> split_pane(PaneId pane, PaneSplitOrientation orientation,
                                    std::string command = "/bin/sh");
    /** @brief Select one live pane in its shared window without changing layout geometry. */
    Result<PaneSnapshot> select_pane(PaneId pane);
    /** @brief Materialize one window's panes in layout order. */
    Result<std::vector<PaneSnapshot>> panes(WindowId window) const;
    /** @brief Store up to 256 KiB under an explicit server-global paste-buffer name.
     *
     * Empty data succeeds without changing the buffer store. */
    Result<void> set_paste_buffer(std::string name, std::string bytes);
    /** @brief Copy arbitrary bytes from one explicitly named server-global paste buffer. */
    Result<std::string> read_paste_buffer(std::string_view name) const;
    /** @brief Delete one explicitly named server-global paste buffer. */
    Result<void> delete_paste_buffer(std::string_view name);
    /** @brief Replace line feeds with carriage returns and atomically queue one named buffer for a
     * pane. */
    Result<void> paste_buffer_into_pane(std::string_view name, PaneId pane);
    /** @copydoc tmux_cxx::ServerConnection::send_pane_input */
    Result<void> send_pane_input(PaneId pane, std::string_view bytes);
    /** @brief Read retained raw PTY output; report a gap if earlier bytes were discarded. */
    Result<std::string> read_pane_output(PaneId pane) const;
    /** @copybrief tmux_cxx::ServerConnection::read_pane_text */
    Result<PaneTextSnapshot> read_pane_text(PaneId pane) const;
    /** @brief Borrow readable PTY descriptors until the next pane mutation or I/O event. */
    std::vector<std::pair<PaneId, int>> pane_pty_descriptors() const;
    /** @brief Advance one pane's input and output within separate 64-KiB byte budgets. */
    void advance_pane_pty_io(PaneId pane);
    /** @brief Report queued program input requiring writable PTY readiness. */
    bool pane_pty_write_pending(PaneId pane) const;
    /** @brief Pause output parsing while earlier terminal replies await input capacity. */
    bool pane_pty_read_blocked(PaneId pane) const;
    /** @brief Reap exited pane children without waiting for running processes. */
    void reap_pane_programs();
    /** @brief Borrow owned child-exit descriptors, including programs whose panes were already
     * removed. */
    std::vector<int> pane_program_readiness_descriptors() const;
    /** @brief Return the earliest owned program-termination deadline for readiness scheduling. */
    std::optional<std::chrono::steady_clock::time_point> next_pane_program_deadline() const;
    /** @brief Close server resources and reap owned programs within 700 milliseconds; report any
     * remaining child explicitly. */
    Result<void> shutdown();

  private:
    friend class StockClientLifecycle;
    friend class ClientTerminalRouting;
    friend class ClientViewRefresh;
    friend class LatestClientWindowSize;
    friend class PaneOutputReader;
    friend class SessionEventReader;
    struct State;
    /** @brief Spawn one preallocated pane at its owner-qualified initial terminal geometry. */
    Result<void> spawn_pane(PaneId pane, WindowId window, terminal::CellSize size,
                            std::string_view command);
    std::unique_ptr<State>
        state_; ///< Exclusive owner of sessions, shared windows, memberships, and pane resources.
};

/** @brief Run the foreground daemon; readiness_descriptor receives R after both listeners bind. */
int run_server(const std::string &stock_socket_path, int readiness_descriptor = -1);
} // namespace tmux_cxx
