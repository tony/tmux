#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/select_pane.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<PaneSnapshot> ServerConnection::select_pane(PaneId pane) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(SelectPane::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto snapshot = reader.pane();
        reader.require_end();
        return snapshot;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(SelectPane::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
