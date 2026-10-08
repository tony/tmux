#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window_link/operations/list_window_links.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::vector<WindowLinkSnapshot>> ServerConnection::window_links(SessionId session) {
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(session.value);
    auto reply = channel_->invoke_operation(ListWindowLinks::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto count = reader.u32();
        if (count > 65536) {
            throw std::invalid_argument("excess window membership count");
        }
        std::vector<WindowLinkSnapshot> links;
        links.reserve(count);
        for (std::uint32_t i = 0; i < count; ++i) {
            links.push_back(reader.window_link());
        }
        reader.require_end();
        return links;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(ListWindowLinks::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
