#include <tmux_cxx/session/operations/list_sessions.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
std::vector<SessionSnapshot> Server::sessions() const {
    std::vector<SessionSnapshot> snapshots;
    for (const auto &[session_id, session] : state_->sessions) {
        (void)session_id;
        snapshots.push_back(state_->session_snapshot(session));
    }
    return snapshots;
}

ListSessions::Request ListSessions::read(protocol::native::NativePayloadReader &) { return {}; }

Result<ListSessions::Response> ListSessions::execute(Server &server, Request) {
    return server.sessions();
}

protocol::native::OperationReply ListSessions::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.size()));
    for (const auto &session : response) {
        reply.session(session);
    }
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
