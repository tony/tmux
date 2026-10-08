#include <tmux_cxx/session/operations/rename_session.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<void> Server::rename_session(SessionId session_id, std::string name) {
    auto session_entry = state_->sessions.find(session_id);
    if (session_entry == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    if (!valid_session_name(name)) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument, "invalid session name"});
    }
    for (const auto &[other_session_id, other_session] : state_->sessions) {
        if (other_session_id != session_id && other_session.name == name) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::duplicate_name, "session name already exists"});
        }
    }
    if (session_entry->second.name == name) {
        return {};
    }
    session_entry->second.name = std::move(name);
    session_entry->second.revision = ++state_->revision;
    state_->session_event_journal.append(SessionRenamedEvent{session_id, session_entry->second.name,
                                                             session_entry->second.revision});
    return {};
}

RenameSession::Request RenameSession::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}, reader.string()};
}

Result<RenameSession::Response> RenameSession::execute(Server &server, Request request) {
    return server.rename_session(request.session, std::move(request.name));
}

protocol::native::OperationReply RenameSession::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
