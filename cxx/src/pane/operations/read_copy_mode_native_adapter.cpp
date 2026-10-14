#include "connection/native_server_channel.hpp"
#include "pane/copy_mode_native_codec.hpp"
#include <tmux_cxx/pane/operations/read_copy_mode.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<CopyModeSnapshot> ServerConnection::copy_mode(PaneId pane) {
    if (pane.value == 0) {
        return std::unexpected(channel_->operation_error(ReadCopyMode::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid copy-mode target"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(ReadCopyMode::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto snapshot = protocol::native::read_copy_mode_snapshot(reader);
        reader.require_end();
        if (snapshot.server_instance != channel_->observed_server_instance() ||
            snapshot.pane != pane) {
            throw std::invalid_argument("copy-mode reply belongs to another server or pane");
        }
        return snapshot;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(channel_->operation_error(ReadCopyMode::description,
                                                         TmuxErrorCode::protocol, error.what()));
    } catch (const std::length_error &error) {
        return std::unexpected(channel_->operation_error(ReadCopyMode::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
