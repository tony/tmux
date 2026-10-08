#include <tmux_cxx/pane/operations/read_pane_output.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <utility>

namespace tmux_cxx {
Result<std::string> Server::read_pane_output(PaneId pane_id) const {
    const auto pane_entry = state_->panes.find(pane_id);
    if (pane_entry == state_->panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    if (pane_entry->second->output.prefix_discarded()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::output_gap, "raw output exceeded retained byte capacity"});
    }
    return std::string(pane_entry->second->output.retained_bytes());
}

ReadPaneOutput::Request ReadPaneOutput::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}};
}

Result<ReadPaneOutput::Response> ReadPaneOutput::execute(Server &server, Request request) {
    return server.read_pane_output(request.pane);
}

protocol::native::OperationReply ReadPaneOutput::write(Response response) {
    protocol::native::NativePayloadWriter writer;
    writer.string(response);
    return std::move(writer.bytes);
}
} // namespace tmux_cxx
