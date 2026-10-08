#include "binding_documentation.hpp"
#include <tmux_cxx/connection/server_connection.hpp>

#include <nanobind/nanobind.h>
#include <nanobind/stl/optional.h>
#include <nanobind/stl/string.h>
#include <nanobind/stl/variant.h>
#include <nanobind/stl/vector.h>

#include <stdexcept>

namespace nb = nanobind;
namespace {
/** @brief Carry a native tmux failure until nanobind translates it while holding the GIL. */
class PythonTmuxError : public std::runtime_error {
  public:
    /** @brief Retain the native classification and request context for Python exception
     * translation. */
    explicit PythonTmuxError(tmux_cxx::TmuxError error)
        : std::runtime_error(error.message), failure(std::move(error)) {}
    tmux_cxx::TmuxError failure; ///< Owned native classification and request context.
};
/** @brief Move a successful native value or preserve its tmux failure as an exception. */
template <class T> T unwrap(tmux_cxx::Result<T> result) {
    if (!result) {
        throw PythonTmuxError(result.error());
    }
    return std::move(*result);
}
/** @brief Translate an unsuccessful mutation without discarding native error context. */
void unwrap(tmux_cxx::Result<void> result) {
    if (!result) {
        throw PythonTmuxError(result.error());
    }
}
} // namespace

/** @brief Export native values and connection operations without starting or connecting a server.
 */
NB_MODULE(_native, module) {
    namespace docs = tmux_cxx::binding_documentation;
    using tmux_cxx::ClientId;
    using tmux_cxx::ClientSnapshot;
    using tmux_cxx::PaneId;
    using tmux_cxx::PaneSnapshot;
    using tmux_cxx::PaneSplitOrientation;
    using tmux_cxx::PaneTextSnapshot;
    using tmux_cxx::ServerConnection;
    using tmux_cxx::SessionClosedEvent;
    using tmux_cxx::SessionCreatedEvent;
    using tmux_cxx::SessionEventBatch;
    using tmux_cxx::SessionEventCursor;
    using tmux_cxx::SessionEventGap;
    using tmux_cxx::SessionEventRecord;
    using tmux_cxx::SessionEventStream;
    using tmux_cxx::SessionId;
    using tmux_cxx::SessionObservation;
    using tmux_cxx::SessionRenamedEvent;
    using tmux_cxx::SessionSnapshot;
    using tmux_cxx::StockConnectionCompatibility;
    using tmux_cxx::WindowLinkSnapshot;
    using tmux_cxx::WindowSnapshot;
    nb::enum_<PaneSplitOrientation>(module, "PaneSplitOrientation")
        .value("LEFT_RIGHT", PaneSplitOrientation::left_right)
        .value("TOP_BOTTOM", PaneSplitOrientation::top_bottom);
    auto tmux_error_type = nb::exception<PythonTmuxError>(module, "TmuxError", PyExc_RuntimeError);
    nb::register_exception_translator(
        /** @brief Attach the native error classification and operation while holding the GIL. */
        [](const std::exception_ptr &error, void *payload) {
            try {
                std::rethrow_exception(error);
            } catch (const PythonTmuxError &failure) {
                auto exception_type = nb::borrow<nb::object>(static_cast<PyObject *>(payload));
                auto python_error = exception_type(failure.what());
                python_error.attr("code") =
                    std::string(tmux_cxx::tmux_error_code_name(failure.failure.code));
                python_error.attr("operation") = failure.failure.operation;
                python_error.attr("server_instance") = failure.failure.server_instance;
                PyErr_SetObject(exception_type.ptr(), python_error.ptr());
            }
        },
        tmux_error_type.ptr());
    nb::class_<SessionSnapshot>(module, "SessionSnapshot", docs::session_type)
        .def_prop_ro(
            "id", /** @brief Expose the materialized session identity without publishing an entity
                     pointer. */
            [](const SessionSnapshot &snapshot) { return snapshot.id.value; }, docs::session_id)
        .def_prop_ro(
            "window", /** @brief Expose the selected window identity from this snapshot. */
            [](const SessionSnapshot &snapshot) { return snapshot.window.value; },
            docs::session_window)
        .def_prop_ro(
            "link", /** @brief Expose the selected session-to-window link identity from this
                       snapshot. */
            [](const SessionSnapshot &snapshot) { return snapshot.link.value; }, docs::session_link)
        .def_prop_ro(
            "pane", /** @brief Expose the selected pane identity from this snapshot. */
            [](const SessionSnapshot &snapshot) { return snapshot.pane.value; }, docs::session_pane)
        .def_ro("name", &SessionSnapshot::name, docs::session_name)
        .def_ro("revision", &SessionSnapshot::revision, docs::session_revision);
    nb::class_<SessionEventCursor>(module, "SessionEventCursor", docs::session_event_cursor_type)
        .def_ro("server_instance", &SessionEventCursor::server_instance,
                docs::session_event_cursor_server_instance)
        .def_ro("next_sequence", &SessionEventCursor::next_sequence,
                docs::session_event_cursor_next_sequence);
    nb::class_<SessionObservation>(module, "SessionObservation", docs::session_observation_type)
        .def_ro("sessions", &SessionObservation::sessions, docs::session_observation_sessions)
        .def_ro("next_event", &SessionObservation::next_event,
                docs::session_observation_next_event);
    nb::class_<SessionCreatedEvent>(module, "SessionCreatedEvent", docs::session_created_event_type)
        .def_ro("session", &SessionCreatedEvent::session, docs::session_created_event_session);
    nb::class_<SessionRenamedEvent>(module, "SessionRenamedEvent", docs::session_renamed_event_type)
        .def_prop_ro(
            "session", /** @brief Expose the renamed session identity as an exact Python integer. */
            [](const SessionRenamedEvent &event) { return event.session.value; },
            docs::session_renamed_event_session)
        .def_ro("name", &SessionRenamedEvent::name, docs::session_renamed_event_name)
        .def_ro("revision", &SessionRenamedEvent::revision, docs::session_renamed_event_revision);
    nb::class_<SessionClosedEvent>(module, "SessionClosedEvent", docs::session_closed_event_type)
        .def_prop_ro(
            "session", /** @brief Expose the removed session identity as an exact Python integer. */
            [](const SessionClosedEvent &event) { return event.session.value; },
            docs::session_closed_event_session)
        .def_ro("name", &SessionClosedEvent::name, docs::session_closed_event_name)
        .def_ro("revision", &SessionClosedEvent::revision, docs::session_closed_event_revision);
    nb::class_<SessionEventRecord>(module, "SessionEventRecord", docs::session_event_record_type)
        .def_ro("sequence", &SessionEventRecord::sequence, docs::session_event_record_sequence)
        .def_ro("event", &SessionEventRecord::event, docs::session_event_record_event);
    nb::class_<SessionEventBatch>(module, "SessionEventBatch", docs::session_event_batch_type)
        .def_ro("records", &SessionEventBatch::records, docs::session_event_batch_records)
        .def_ro("next_cursor", &SessionEventBatch::next_cursor,
                docs::session_event_batch_next_cursor);
    nb::class_<SessionEventGap>(module, "SessionEventGap", docs::session_event_gap_type)
        .def_ro("missed_from", &SessionEventGap::missed_from, docs::session_event_gap_missed_from)
        .def_ro("replacement", &SessionEventGap::replacement, docs::session_event_gap_replacement);
    nb::class_<SessionEventStream>(module, "SessionEventStream", docs::session_event_stream_type)
        .def_prop_ro("initial", &SessionEventStream::initial, docs::session_event_stream_initial)
        .def(
            "next", /** @copybrief tmux_cxx::SessionEventStream::next */
            [](SessionEventStream &self, std::uint32_t timeout_ms) {
                return unwrap(self.next(timeout_ms));
            },
            nb::arg("timeout_ms") = 500, docs::session_event_stream_next,
            nb::call_guard<nb::gil_scoped_release>())
        .def("close", &SessionEventStream::close, docs::session_event_stream_close)
        .def(
            "__enter__", /** @brief Borrow this stream without transferring its cursor. */
            [](SessionEventStream &self) -> SessionEventStream & { return self; },
            nb::rv_policy::reference,
            "Borrow this stream without transferring its observer-owned cursor.")
        .def(
            "__exit__", /** @brief Close this stream without suppressing the context's exception. */
            [](SessionEventStream &self, const nb::object &, const nb::object &,
               const nb::object &) { self.close(); },
            nb::arg("exc_type").none(), nb::arg("exc_value").none(), nb::arg("traceback").none(),
            "Close the stream without suppressing the context's exception.");
    nb::class_<WindowSnapshot>(module, "WindowSnapshot", docs::window_type)
        .def_prop_ro(
            "id", /** @brief Expose this snapshot's shared window identity as an exact Python
                     integer. */
            [](const WindowSnapshot &snapshot) { return snapshot.id.value; }, docs::window_id)
        .def_prop_ro(
            "pane", /** @brief Expose the active pane selected by the shared window. */
            [](const WindowSnapshot &snapshot) { return snapshot.pane.value; }, docs::window_pane)
        .def_ro("name", &WindowSnapshot::name, docs::window_name)
        .def_ro("rows", &WindowSnapshot::rows, docs::window_rows)
        .def_ro("columns", &WindowSnapshot::columns, docs::window_columns)
        .def_ro("revision", &WindowSnapshot::revision, docs::window_revision);
    nb::class_<WindowLinkSnapshot>(module, "WindowLinkSnapshot", docs::window_link_type)
        .def_prop_ro(
            "id", /** @brief Expose the membership identity without publishing a server pointer. */
            [](const WindowLinkSnapshot &snapshot) { return snapshot.id.value; },
            docs::window_link_id)
        .def_prop_ro(
            "session", /** @brief Expose the session owning this membership's index. */
            [](const WindowLinkSnapshot &snapshot) { return snapshot.session.value; },
            docs::window_link_session)
        .def_prop_ro(
            "window", /** @brief Expose the shared window retained by this membership. */
            [](const WindowLinkSnapshot &snapshot) { return snapshot.window.value; },
            docs::window_link_window)
        .def_ro("index", &WindowLinkSnapshot::index, docs::window_link_index)
        .def_ro("revision", &WindowLinkSnapshot::revision, docs::window_link_revision);
    nb::class_<PaneSnapshot>(module, "PaneSnapshot", docs::pane_type)
        .def_prop_ro(
            "id", /** @brief Expose the pane identity without publishing a live entity pointer. */
            [](const PaneSnapshot &snapshot) { return snapshot.id.value; }, docs::pane_id)
        .def_prop_ro(
            "window", /** @brief Expose the shared window that owns this pane. */
            [](const PaneSnapshot &snapshot) { return snapshot.window.value; }, docs::pane_window)
        .def_ro("top", &PaneSnapshot::top, docs::pane_top)
        .def_ro("left", &PaneSnapshot::left, docs::pane_left)
        .def_ro("rows", &PaneSnapshot::rows, docs::pane_rows)
        .def_ro("columns", &PaneSnapshot::columns, docs::pane_columns)
        .def_ro("active", &PaneSnapshot::active, docs::pane_active)
        .def_ro("revision", &PaneSnapshot::revision, docs::pane_revision);
    nb::class_<PaneTextSnapshot>(module, "PaneTextSnapshot", docs::pane_text_type)
        .def_prop_ro(
            "pane", /** @brief Expose the observed pane identity without borrowing a server entity.
                     */
            [](const PaneTextSnapshot &snapshot) { return snapshot.pane.value; },
            docs::pane_text_pane)
        .def_ro("server_instance", &PaneTextSnapshot::server_instance,
                docs::pane_text_server_instance)
        .def_ro("revision", &PaneTextSnapshot::revision, docs::pane_text_revision)
        .def_ro("rows", &PaneTextSnapshot::rows, docs::pane_text_rows)
        .def_ro("columns", &PaneTextSnapshot::columns, docs::pane_text_columns)
        .def_ro("cursor_row", &PaneTextSnapshot::cursor_row, docs::pane_text_cursor_row)
        .def_ro("cursor_column", &PaneTextSnapshot::cursor_column, docs::pane_text_cursor_column)
        .def_ro("cursor_visible", &PaneTextSnapshot::cursor_visible, docs::pane_text_cursor_visible)
        .def_ro("alternate_screen", &PaneTextSnapshot::alternate_screen,
                docs::pane_text_alternate_screen)
        .def_ro("complete", &PaneTextSnapshot::complete, docs::pane_text_complete)
        .def_ro("lines", &PaneTextSnapshot::lines, docs::pane_text_lines)
        .def_ro("history_lines", &PaneTextSnapshot::history_lines, docs::pane_text_history_lines)
        .def_ro("history_gap", &PaneTextSnapshot::history_gap, docs::pane_text_history_gap)
        .def_ro("input_open", &PaneTextSnapshot::input_open, docs::pane_text_input_open)
        .def_ro("input_pending", &PaneTextSnapshot::input_pending, docs::pane_text_input_pending);
    nb::class_<ClientSnapshot>(module, "ClientSnapshot", docs::client_type)
        .def_prop_ro(
            "id", /** @brief Expose this connection-lifetime client identity as an exact integer. */
            [](const ClientSnapshot &snapshot) { return snapshot.id.value; }, docs::client_id)
        .def_prop_ro(
            "session", /** @brief Expose the attached session identity or None while detached. */
            [](const ClientSnapshot &snapshot) -> std::optional<std::uint64_t> {
                if (snapshot.session) {
                    return snapshot.session->value;
                }
                return std::nullopt;
            },
            docs::client_session)
        .def_ro("detached_from", &ClientSnapshot::detached_from, docs::client_detached_from)
        .def_ro("identified", &ClientSnapshot::identified, docs::client_identified)
        .def_ro("has_terminal", &ClientSnapshot::has_terminal, docs::client_has_terminal)
        .def_ro("rows", &ClientSnapshot::rows, docs::client_rows)
        .def_ro("columns", &ClientSnapshot::columns, docs::client_columns)
        .def_ro("revision", &ClientSnapshot::revision, docs::client_revision);
    nb::class_<StockConnectionCompatibility>(module, "StockConnectionCompatibility",
                                             docs::stock_connection_compatibility_type)
        .def_ro("profile", &StockConnectionCompatibility::profile,
                docs::stock_connection_compatibility_profile)
        .def_ro("release", &StockConnectionCompatibility::release,
                docs::stock_connection_compatibility_release)
        .def_ro("release_uncertain", &StockConnectionCompatibility::release_uncertain,
                docs::stock_connection_compatibility_release_uncertain)
        .def_prop_ro(
            "direction",
            /** @brief Expose the stable stock adapter role instead of its native enum value. */
            [](const StockConnectionCompatibility &compatibility) {
                return std::string(
                    tmux_cxx::protocol::tmux::protocol_direction_name(compatibility.direction));
            },
            docs::stock_connection_compatibility_direction)
        .def_prop_ro(
            "mode", /** @brief Expose the stable stock connection mode spelling. */
            [](const StockConnectionCompatibility &compatibility) {
                return std::string(
                    tmux_cxx::protocol::tmux::connection_mode_name(compatibility.mode));
            },
            docs::stock_connection_compatibility_mode)
        .def_prop_ro(
            "tier", /** @brief Expose the stable release-support tier spelling. */
            [](const StockConnectionCompatibility &compatibility) {
                return std::string(tmux_cxx::protocol::tmux::support_tier_name(compatibility.tier));
            },
            docs::stock_connection_compatibility_tier)
        .def_prop_ro(
            "support", /** @brief Expose effective capability fidelity as stable text. */
            [](const StockConnectionCompatibility &compatibility) {
                return std::string(
                    tmux_cxx::protocol::tmux::capability_support_name(compatibility.support));
            },
            docs::stock_connection_compatibility_support)
        .def_prop_ro(
            "verification", /** @brief Expose compatibility evidence strength as stable text. */
            [](const StockConnectionCompatibility &compatibility) {
                return std::string(tmux_cxx::protocol::tmux::compatibility_verification_name(
                    compatibility.verification));
            },
            docs::stock_connection_compatibility_verification)
        .def_ro("commands", &StockConnectionCompatibility::commands,
                docs::stock_connection_compatibility_commands)
        .def_ro("quirks", &StockConnectionCompatibility::quirks,
                docs::stock_connection_compatibility_quirks)
        .def_ro("evidence", &StockConnectionCompatibility::evidence,
                docs::stock_connection_compatibility_evidence)
        .def_ro("limitations", &StockConnectionCompatibility::limitations,
                docs::stock_connection_compatibility_limitations);
    nb::class_<ServerConnection>(module, "ServerConnection")
        .def(nb::init<std::string>(), nb::arg("socket"), docs::connection)
        .def(
            "create_session", /** @copybrief tmux_cxx::ServerConnection::create_session */
            [](ServerConnection &self, std::string name, std::string command) {
                return unwrap(self.create_session(std::move(name), std::move(command)));
            },
            nb::arg("name"), nb::arg("command") = "/bin/sh", docs::create_session,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "sessions", /** @copybrief tmux_cxx::ServerConnection::sessions */
            [](ServerConnection &self) { return unwrap(self.sessions()); }, docs::sessions,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "observe_sessions", /** @copybrief tmux_cxx::ServerConnection::observe_sessions */
            [](ServerConnection &self) { return unwrap(self.observe_sessions()); },
            docs::observe_sessions, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "subscribe_sessions", /** @copybrief tmux_cxx::ServerConnection::subscribe_sessions */
            [](ServerConnection &self) { return unwrap(self.subscribe_sessions()); },
            docs::subscribe_sessions, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "read_session_events", /** @copybrief tmux_cxx::ServerConnection::read_session_events */
            [](ServerConnection &self, SessionEventCursor cursor) {
                return unwrap(self.read_session_events(std::move(cursor)));
            },
            nb::arg("cursor"), docs::read_session_events, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "wait_session_events", /** @copybrief tmux_cxx::ServerConnection::wait_session_events */
            [](ServerConnection &self, SessionEventCursor cursor, std::uint32_t timeout_ms) {
                return unwrap(self.wait_session_events(std::move(cursor), timeout_ms));
            },
            nb::arg("cursor"), nb::arg("timeout_ms") = 500, docs::wait_session_events,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "rename_session", /** @copybrief tmux_cxx::ServerConnection::rename_session */
            [](ServerConnection &self, std::uint64_t id, std::string name) {
                unwrap(self.rename_session(SessionId{id}, std::move(name)));
            },
            nb::arg("id"), nb::arg("name"), docs::rename_session,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "kill_session", /** @copybrief tmux_cxx::ServerConnection::kill_session */
            [](ServerConnection &self, std::uint64_t id) {
                unwrap(self.kill_session(SessionId{id}));
            },
            nb::arg("id"), docs::kill_session, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "clients", /** @copybrief tmux_cxx::ServerConnection::clients */
            [](ServerConnection &self) { return unwrap(self.clients()); }, docs::clients,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "stock_client_compatibility",
            /** @copybrief tmux_cxx::ServerConnection::stock_client_compatibility */
            [](ServerConnection &self, std::uint64_t client) {
                return unwrap(self.stock_client_compatibility(ClientId{client}));
            },
            nb::arg("client"), docs::stock_client_compatibility,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "attach_client", /** @copybrief tmux_cxx::ServerConnection::attach_client */
            [](ServerConnection &self, std::uint64_t client, std::uint64_t session) {
                return unwrap(self.attach_client(ClientId{client}, SessionId{session}));
            },
            nb::arg("client"), nb::arg("session"), docs::attach_client,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "detach_client", /** @copybrief tmux_cxx::ServerConnection::detach_client */
            [](ServerConnection &self, std::uint64_t client) {
                return unwrap(self.detach_client(ClientId{client}));
            },
            nb::arg("client"), docs::detach_client, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "resize_client", /** @copybrief tmux_cxx::ServerConnection::resize_client */
            [](ServerConnection &self, std::uint64_t client) {
                return unwrap(self.resize_client(ClientId{client}));
            },
            nb::arg("client"), docs::resize_client, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "link_window", /** @copybrief tmux_cxx::ServerConnection::link_window */
            [](ServerConnection &self, std::uint64_t window, std::uint64_t session,
               std::uint32_t index) {
                return unwrap(
                    self.link_window(tmux_cxx::WindowId{window}, SessionId{session}, index));
            },
            nb::arg("window"), nb::arg("session"), nb::arg("index"), docs::link_window,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "unlink_window", /** @copybrief tmux_cxx::ServerConnection::unlink_window */
            [](ServerConnection &self, std::uint64_t id) {
                unwrap(self.unlink_window(tmux_cxx::WindowLinkId{id}));
            },
            nb::arg("id"), docs::unlink_window, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "window_links", /** @copybrief tmux_cxx::ServerConnection::window_links */
            [](ServerConnection &self, std::uint64_t session) {
                return unwrap(self.window_links(SessionId{session}));
            },
            nb::arg("session"), docs::window_links, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "window", /** @copybrief tmux_cxx::ServerConnection::window */
            [](ServerConnection &self, std::uint64_t id) {
                return unwrap(self.window(tmux_cxx::WindowId{id}));
            },
            nb::arg("id"), docs::window, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "split_pane", /** @copybrief tmux_cxx::ServerConnection::split_pane */
            [](ServerConnection &self, std::uint64_t pane, PaneSplitOrientation orientation,
               std::string command) {
                return unwrap(self.split_pane(PaneId{pane}, orientation, std::move(command)));
            },
            nb::arg("pane"), nb::arg("orientation"), nb::arg("command") = "/bin/sh",
            docs::split_pane, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "select_pane", /** @copybrief tmux_cxx::ServerConnection::select_pane */
            [](ServerConnection &self, std::uint64_t pane) {
                return unwrap(self.select_pane(PaneId{pane}));
            },
            nb::arg("pane"), docs::select_pane, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "panes", /** @copybrief tmux_cxx::ServerConnection::panes */
            [](ServerConnection &self, std::uint64_t window) {
                return unwrap(self.panes(tmux_cxx::WindowId{window}));
            },
            nb::arg("window"), docs::panes, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "set_paste_buffer", /** @brief Copy arbitrary bytes before releasing the GIL for
                                    server-global storage. */
            [](ServerConnection &self, std::string name, nb::bytes python_bytes) {
                std::string bytes(python_bytes.c_str(), python_bytes.size());
                nb::gil_scoped_release release;
                unwrap(self.set_paste_buffer(std::move(name), std::move(bytes)));
            },
            nb::arg("name"), nb::arg("bytes"), docs::set_paste_buffer)
        .def(
            "read_paste_buffer", /** @brief Release the GIL for buffer observation and reacquire it
                                     before constructing Python bytes. */
            [](ServerConnection &self, std::string name) {
                std::string bytes;
                {
                    nb::gil_scoped_release release;
                    bytes = unwrap(self.read_paste_buffer(std::move(name)));
                }
                return nb::bytes(bytes.data(), bytes.size());
            },
            nb::arg("name"), docs::read_paste_buffer)
        .def(
            "delete_paste_buffer", /** @copybrief tmux_cxx::ServerConnection::delete_paste_buffer */
            [](ServerConnection &self, std::string name) {
                unwrap(self.delete_paste_buffer(std::move(name)));
            },
            nb::arg("name"), docs::delete_paste_buffer, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "paste_buffer_into_pane",
            /** @copybrief tmux_cxx::ServerConnection::paste_buffer_into_pane */
            [](ServerConnection &self, std::string name, std::uint64_t pane) {
                unwrap(self.paste_buffer_into_pane(std::move(name), PaneId{pane}));
            },
            nb::arg("name"), nb::arg("pane"), docs::paste_buffer_into_pane,
            nb::call_guard<nb::gil_scoped_release>())
        .def(
            "send_pane_input", /** @brief Copy raw bytes before releasing the GIL to deliver program
                                  input. */
            [](ServerConnection &self, std::uint64_t pane, nb::bytes python_bytes) {
                std::string bytes(python_bytes.c_str(), python_bytes.size());
                nb::gil_scoped_release release;
                unwrap(self.send_pane_input(PaneId{pane}, std::move(bytes)));
            },
            nb::arg("pane"), nb::arg("bytes"), docs::send_pane_input)
        .def(
            "read_pane_output", /** @brief Release the GIL for native observation and reacquire it
                                   before constructing Python bytes. */
            [](ServerConnection &self, std::uint64_t pane) {
                std::string bytes;
                {
                    nb::gil_scoped_release release;
                    bytes = unwrap(self.read_pane_output(PaneId{pane}));
                }
                return nb::bytes(bytes.data(), bytes.size());
            },
            nb::arg("pane"), docs::read_pane_output)
        .def(
            "read_pane_text", /** @copybrief tmux_cxx::ServerConnection::read_pane_text */
            [](ServerConnection &self, std::uint64_t id) {
                return unwrap(self.read_pane_text(PaneId{id}));
            },
            nb::arg("id"), docs::read_pane_text, nb::call_guard<nb::gil_scoped_release>())
        .def(
            "wait_pane_output", /** @brief Copy the byte pattern, release the GIL while waiting, and
                                   materialize bytes after reacquisition. */
            [](ServerConnection &self, std::uint64_t pane, nb::bytes needle,
               std::uint32_t timeout_ms) {
                std::string pattern(needle.c_str(), needle.size());
                std::string bytes;
                {
                    nb::gil_scoped_release release;
                    bytes =
                        unwrap(self.wait_pane_output(PaneId{pane}, std::move(pattern), timeout_ms));
                }
                return nb::bytes(bytes.data(), bytes.size());
            },
            nb::arg("pane"), nb::arg("needle"), nb::arg("timeout_ms") = 500, docs::wait_pane_output)
        .def("close", &ServerConnection::close, docs::close)
        .def(
            "__enter__", /** @brief Borrow this connection for a context without transferring server
                            ownership. */
            [](ServerConnection &self) -> ServerConnection & { return self; },
            nb::rv_policy::reference,
            "Borrow this connection for a context without transferring server ownership.")
        .def(
            "__exit__", /** @brief Close request admission without suppressing the context's
                           exception or destroying sessions. */
            [](ServerConnection &self, const nb::object &, const nb::object &, const nb::object &) {
                self.close();
            },
            nb::arg("exc_type").none(), nb::arg("exc_value").none(), nb::arg("traceback").none(),
            "Close admission without suppressing exceptions or destroying persistent sessions.");
}
