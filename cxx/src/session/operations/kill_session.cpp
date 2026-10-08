#include <tmux_cxx/session/operations/kill_session.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<void> Server::kill_session(SessionId session_id) {
    auto session_entry = state_->sessions.find(session_id);
    if (session_entry == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    const auto session_name = session_entry->second.name;
    std::vector<ClientId> attached_clients;
    for (const auto &[client_id, client] : state_->clients) {
        if (client->session == session_id) {
            attached_clients.push_back(client_id);
        }
    }
    for (const auto client_id : attached_clients) {
        if (auto detachment = detach_client(client_id); !detachment) {
            return std::unexpected(detachment.error());
        }
    }
    while (!session_entry->second.windows.empty()) {
        state_->remove_window_link(session_entry->second.windows.begin()->second);
    }
    state_->sessions.erase(session_entry);
    const auto revision = ++state_->revision;
    state_->session_event_journal.append(SessionClosedEvent{session_id, session_name, revision});
    return {};
}

KillSession::Request KillSession::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}};
}

Result<KillSession::Response> KillSession::execute(Server &server, Request request) {
    return server.kill_session(request.session);
}

protocol::native::OperationReply KillSession::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
