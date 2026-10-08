#include "session/session_event_reader.hpp"

#include "server/state.hpp"
#include <tmux_cxx/server/server.hpp>

#include <utility>

namespace tmux_cxx {
SessionObservation SessionEventReader::observe_sessions(const Server &server) {
    std::vector<SessionSnapshot> sessions;
    sessions.reserve(server.state_->sessions.size());
    for (const auto &[session_id, session] : server.state_->sessions) {
        (void)session_id;
        sessions.push_back(server.state_->session_snapshot(session));
    }
    return {std::move(sessions), tail(server)};
}

SessionEventCursor SessionEventReader::tail(const Server &server) {
    return {server.state_->server_instance, server.state_->session_event_journal.tail_sequence()};
}

Result<SessionEventBatch> SessionEventReader::read(const Server &server,
                                                   const SessionEventCursor &cursor) {
    if (cursor.server_instance != server.state_->server_instance) {
        return std::unexpected(TmuxError{TmuxErrorCode::wrong_server,
                                         "session event cursor belongs to another server",
                                         {},
                                         server.state_->server_instance});
    }
    auto event_records = server.state_->session_event_journal.read_from(cursor.next_sequence);
    if (!event_records) {
        auto error = std::move(event_records.error());
        error.server_instance = server.state_->server_instance;
        return std::unexpected(std::move(error));
    }
    return SessionEventBatch{
        std::move(*event_records),
        {server.state_->server_instance, server.state_->session_event_journal.tail_sequence()}};
}
} // namespace tmux_cxx
