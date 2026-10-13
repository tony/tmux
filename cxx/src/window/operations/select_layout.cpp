#include <tmux_cxx/window/operations/select_layout.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"

#include <algorithm>
#include <stdexcept>
#include <utility>

namespace tmux_cxx {
Result<WindowSnapshot> Server::select_layout(WindowId window_id, WindowLayoutPreset preset) {
    const auto entry = state_->windows.find(window_id);
    if (entry == state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    auto &window_state = entry->second;
    auto candidate = window_state.layout;
    const auto changed = candidate.arrange(preset, window_state.pane_order);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (*changed) {
        if (auto resized =
                state_->resize_pane_terminals(candidate.panes(), window_state.layout.panes());
            !resized) {
            return std::unexpected(resized.error());
        }
        window_state.size = candidate.size();
        window_state.layout = std::move(candidate);
        window_state.revision = ++state_->revision;
    }
    return window(window_id);
}

SelectLayout::Request SelectLayout::read(protocol::native::NativePayloadReader &reader) {
    const auto window = WindowId{reader.u64()};
    const auto value = reader.u16();
    const auto preset =
        std::ranges::find(window_layout_presets, value,
                          /** @brief Compare the wire number with a declared preset identity. */
                          [](const WindowLayoutPresetDescription &description) {
                              return std::to_underlying(description.preset);
                          });
    if (preset == window_layout_presets.end()) {
        throw std::invalid_argument("invalid window layout preset");
    }
    return {window, preset->preset};
}

Result<SelectLayout::Response> SelectLayout::execute(Server &server, Request request) {
    return server.select_layout(request.window, request.preset);
}

protocol::native::OperationReply SelectLayout::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.window(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
