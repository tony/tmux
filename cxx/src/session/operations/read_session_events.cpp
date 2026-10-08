#include <tmux_cxx/session/operations/read_session_events.hpp>

#include "protocol/native/framing.hpp"
#include "protocol/native/session_event_codec.hpp"
#include "session/session_event_reader.hpp"

namespace tmux_cxx {
ReadSessionEvents::Request ReadSessionEvents::read(protocol::native::NativePayloadReader &reader) {
    return {{reader.string(), reader.u64()}};
}

Result<ReadSessionEvents::Response> ReadSessionEvents::execute(Server &server, Request request) {
    return SessionEventReader::read(server, request.cursor);
}

protocol::native::OperationReply ReadSessionEvents::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    protocol::native::write_session_event_batch(reply, response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
