#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/wait_pane_output.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::string> ServerConnection::wait_pane_output(PaneId pane, std::string needle,
                                                       std::uint32_t timeout_ms) try {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.string(needle);
    arguments.u32(timeout_ms);
    auto reply = channel_->invoke_operation(WaitPaneOutput::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto raw_output = reader.string();
        if (!reader.finished()) {
            throw std::invalid_argument("excess wait reply bytes");
        }
        return raw_output;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(WaitPaneOutput::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        WaitPaneOutput::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
