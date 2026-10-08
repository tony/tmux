#include "connection/native_server_channel.hpp"
#include <tmux_cxx/pane/operations/read_pane_output.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::string> ServerConnection::read_pane_output(PaneId pane) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    auto reply = channel_->invoke_operation(ReadPaneOutput::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto raw_output = reader.string();
        if (!reader.finished()) {
            throw std::invalid_argument("excess read_pane_output reply bytes");
        }
        return raw_output;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ReadPaneOutput::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
