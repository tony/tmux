#include <tmux_cxx/session/operations/observe_sessions.hpp>

#include "protocol/native/framing.hpp"
#include "session/session_event_reader.hpp"

namespace tmux_cxx {
ObserveSessions::Request ObserveSessions::read(protocol::native::NativePayloadReader &) {
    return {};
}

Result<ObserveSessions::Response> ObserveSessions::execute(Server &server, Request) {
    return SessionEventReader::observe_sessions(server);
}

protocol::native::OperationReply ObserveSessions::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.u32(static_cast<std::uint32_t>(response.sessions.size()));
    for (const auto &session : response.sessions) {
        reply.session(session);
    }
    reply.string(response.next_event.server_instance);
    reply.u64(response.next_event.next_sequence);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
