#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/select_layout.hpp>

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<WindowSnapshot> ServerConnection::select_layout(WindowId window, WindowLayoutPreset preset) {
    if (std::ranges::find(window_layout_presets, preset, &WindowLayoutPresetDescription::preset) ==
        window_layout_presets.end()) {
        return std::unexpected(channel_->operation_error(SelectLayout::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid window layout preset"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(window.value);
    arguments.u16(std::to_underlying(preset));
    auto reply = channel_->invoke_operation(SelectLayout::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto snapshot = reader.window();
        reader.require_end();
        return snapshot;
    } catch (const std::exception &error) {
        return std::unexpected(channel_->operation_error(SelectLayout::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
