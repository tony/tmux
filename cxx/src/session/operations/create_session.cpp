#include <tmux_cxx/session/operations/create_session.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<SessionSnapshot> Server::create_session(std::string name, std::string command) {
    if (!valid_session_name(name) || command.empty() || command.find('\0') != command.npos) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid session name or command"});
    }
    for (const auto &[existing_session_id, session] : state_->sessions) {
        (void)existing_session_id;
        if (session.name == name) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::duplicate_name, "session name already exists"});
        }
    }
    const SessionId session_id{state_->next_session_identity++};
    const WindowId window_id{state_->next_window_identity++};
    const WindowLinkId link_id{state_->next_window_link_identity++};
    const PaneId pane_id{state_->next_pane_identity++};
    constexpr terminal::CellSize initial_size{24, 80};
    if (auto spawned_pane = spawn_pane(pane_id, window_id, initial_size, command); !spawned_pane) {
        return std::unexpected(spawned_pane.error());
    }
    const auto revision = ++state_->revision;
    state_->windows.emplace(window_id, Window{window_id,
                                              name,
                                              WindowLayout{pane_id, initial_size},
                                              pane_id,
                                              {link_id},
                                              initial_size,
                                              WindowSizePolicy::latest,
                                              revision});
    state_->window_links.emplace(link_id, WindowLink{link_id, session_id, window_id, 0, revision});
    const auto session_entry =
        state_->sessions
            .emplace(session_id,
                     Session{session_id, std::move(name), {{0, link_id}}, link_id, {}, revision})
            .first;
    auto snapshot = state_->session_snapshot(session_entry->second);
    state_->session_event_journal.append(SessionCreatedEvent{snapshot});
    return snapshot;
}

CreateSession::Request CreateSession::read(protocol::native::NativePayloadReader &reader) {
    return {reader.string(), reader.string()};
}

Result<CreateSession::Response> CreateSession::execute(Server &server, Request request) {
    return server.create_session(std::move(request.name), std::move(request.command));
}

protocol::native::OperationReply CreateSession::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.session(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
