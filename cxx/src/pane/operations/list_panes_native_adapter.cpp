#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/list_panes.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::vector<PaneSnapshot>> ServerConnection::panes(WindowId window) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window.value);
    auto reply = channel_->invoke_operation(ListPanes::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto count = reader.u32();
        if (count > 256) {
            throw std::invalid_argument("pane list exceeds capacity");
        }
        std::vector<PaneSnapshot> snapshots;
        snapshots.reserve(count);
        for (std::uint32_t index = 0; index < count; ++index) {
            snapshots.push_back(reader.pane());
        }
        reader.require_end();
        return snapshots;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ListPanes::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
