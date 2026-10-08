#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/split_pane.hpp>

#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<PaneSnapshot> ServerConnection::split_pane(PaneId pane, PaneSplitOrientation orientation,
                                                  std::string command) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.u16(std::to_underlying(orientation));
    arguments.string(command);
    auto reply = channel_->invoke_operation(SplitPane::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto snapshot = reader.pane();
        reader.require_end();
        return snapshot;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(SplitPane::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        SplitPane::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
