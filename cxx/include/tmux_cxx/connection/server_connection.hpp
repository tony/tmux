#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/pane/pane_text_snapshot.hpp>
#include <tmux_cxx/pane/split_orientation.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/session/session_event.hpp>
#include <tmux_cxx/session/session_event_stream.hpp>
#include <tmux_cxx/session/session_observation.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

#include <memory>
#include <stop_token>
#include <vector>

namespace tmux_cxx {
class NativeServerChannel;
/** @brief Control a persistent C++ server through shared native admission without owning sessions.
 */
class ServerConnection {
  public:
    /** @brief Bind native requests to the named server endpoint without starting a daemon. */
    explicit ServerConnection(std::string socket_path);
    /** @brief Release one local channel reference without destroying persistent tmux entities. */
    ~ServerConnection();
    /** @brief Prevent copying request-admission and server-identity state. */
    ServerConnection(const ServerConnection &) = delete;
    /** @brief Prevent replacing request-admission state through copy assignment. */
    ServerConnection &operator=(const ServerConnection &) = delete;
    /** @brief Create a persistent PTY-backed session with a valid UTF-8 name on the connected
     * server.
     *
     * @see tmux_cxx::Server::create_session for shared name constraints. */
    Result<SessionSnapshot> create_session(std::string name, std::string command = "/bin/sh");
    /** @brief Materialize session values from the connected server. */
    Result<std::vector<SessionSnapshot>> sessions();
    /** @brief Materialize sessions and the first committed event after their snapshot boundary. */
    Result<SessionObservation> observe_sessions();
    /** @brief Establish an atomic session baseline and an independently cancellable event stream.
     */
    Result<SessionEventStream> subscribe_sessions();
    /** @brief Read retained committed events from an owner-qualified cursor. */
    Result<SessionEventBatch> read_session_events(SessionEventCursor cursor);
    /** @brief Await retained events for 1..750 milliseconds, returning an empty batch on expiry. */
    Result<SessionEventBatch> wait_session_events(SessionEventCursor cursor,
                                                  std::uint32_t timeout_ms = 500);
    /** @brief Rename a session resolved against the current server instance. */
    Result<void> rename_session(SessionId session, std::string name);
    /** @brief Explicitly remove a session; closing this connection never invokes this operation. */
    Result<void> kill_session(SessionId session);
    /** @brief Materialize connection-lifetime clients in stable identity order. */
    Result<std::vector<ClientSnapshot>> clients();
    /** @brief Copy one stock client's selected wire profile, evidence, support, and limits. */
    Result<protocol::tmux::StockConnectionCompatibility>
    stock_client_compatibility(ClientId client);
    /** @brief Attach an identified ordinary client to a retained session. */
    Result<ClientSnapshot> attach_client(ClientId client, SessionId session);
    /** @brief End one client's attachment while retaining both client and session. */
    Result<ClientSnapshot> detach_client(ClientId client);
    /** @brief Remeasure an attached client's retained TTY and resize its selected window. */
    Result<ClientSnapshot> resize_client(ClientId client);
    /** @brief Link a shared window at an unused index in the destination session. */
    Result<WindowLinkSnapshot> link_window(WindowId window, SessionId session, std::uint32_t index);
    /** @brief Remove one window membership while preserving windows retained by other memberships.
     */
    Result<void> unlink_window(WindowLinkId window_link);
    /** @brief Materialize a session's window memberships in index order. */
    Result<std::vector<WindowLinkSnapshot>> window_links(SessionId session);
    /** @brief Materialize shared window state independently of its session memberships. */
    Result<WindowSnapshot> window(WindowId window);
    /** @brief Split one pane, start its command, and materialize the selected new pane. */
    Result<PaneSnapshot> split_pane(PaneId pane, PaneSplitOrientation orientation,
                                    std::string command = "/bin/sh");
    /** @brief Select one live pane in its shared window and materialize the result. */
    Result<PaneSnapshot> select_pane(PaneId pane);
    /** @brief Materialize a shared window's panes in layout order. */
    Result<std::vector<PaneSnapshot>> panes(WindowId window);
    /** @brief Store up to 256 KiB under an explicit server-global paste-buffer name. */
    Result<void> set_paste_buffer(std::string name, std::string bytes);
    /** @brief Copy arbitrary bytes from one explicitly named server-global paste buffer. */
    Result<std::string> read_paste_buffer(std::string name);
    /** @brief Delete one explicitly named server-global paste buffer. */
    Result<void> delete_paste_buffer(std::string name);
    /** @brief Queue one named buffer for a pane using tmux's default line separator. */
    Result<void> paste_buffer_into_pane(std::string name, PaneId pane);
    /** @brief Queue up to 64 KiB of input; accepted bytes may be delivered after completion.
     *
     * Backpressure rejects all request bytes before effects. */
    Result<void> send_pane_input(PaneId pane, std::string bytes);
    /** @brief Read retained raw pane output and report discarded history as a gap. */
    Result<std::string> read_pane_output(PaneId pane);
    /** @brief Copy pane text with terminal freshness and program-input state, separately from raw
     * bytes. */
    Result<PaneTextSnapshot> read_pane_text(PaneId pane);
    /** @brief Await a nonempty byte pattern with a deadline of 1..750 milliseconds. */
    Result<std::string> wait_pane_output(PaneId pane, std::string needle,
                                         std::uint32_t timeout_ms = 500);
    /** @brief Stop admission for this connection and streams sharing its native channel. */
    void close();

  private:
    /** @brief Share an existing native channel with one session-event stream. */
    explicit ServerConnection(std::shared_ptr<NativeServerChannel> channel);
    /** @brief Observe sessions unless the caller cancels its scoped native request. */
    Result<SessionObservation> observe_sessions_until_cancelled(std::stop_token stop_token);
    /** @brief Await session events unless the caller cancels its scoped native request. */
    Result<SessionEventBatch> wait_session_events_until_cancelled(SessionEventCursor cursor,
                                                                  std::uint32_t timeout_ms,
                                                                  std::stop_token stop_token);
    friend class SessionEventStream;
    std::shared_ptr<NativeServerChannel>
        channel_; ///< Shared native admission, negotiation, and observed server identity.
};
} // namespace tmux_cxx
