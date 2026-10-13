#include <tmux_cxx/window/operations/read_window_options.hpp>

#include "server/state.hpp"
#include "window/window_option_target.hpp"
#include "window/window_options.hpp"

namespace tmux_cxx {
Result<WindowOptionsSnapshot> Server::window_options(WindowOptionTarget target) const {
    if (const auto *window_id = std::get_if<WindowId>(&target)) {
        if (window_id->value == 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "window identity must be nonzero"});
        }
        const auto window = state_->windows.find(*window_id);
        if (window == state_->windows.end()) {
            return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
        }
        const auto mode = window_options::effective_mode_keys(window->second.options,
                                                              state_->global_window_options);
        if (!mode) {
            return std::unexpected(mode.error());
        }
        return WindowOptionsSnapshot{state_->server_instance, *window_id, *mode,
                                     window->second.revision};
    }
    const auto mode = window_options::effective_mode_keys({}, state_->global_window_options);
    if (!mode) {
        return std::unexpected(mode.error());
    }
    return WindowOptionsSnapshot{state_->server_instance, std::nullopt, *mode, state_->revision};
}

Result<WindowModeKeys> Server::window_mode_keys(WindowId window_id) const {
    if (window_id.value == 0) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "window identity must be nonzero"});
    }
    const auto window = state_->windows.find(window_id);
    if (window == state_->windows.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
    }
    return window_options::effective_mode_keys(window->second.options,
                                               state_->global_window_options);
}

ReadWindowOptions::Request ReadWindowOptions::read(protocol::native::NativePayloadReader &reader) {
    return {read_window_option_target(reader)};
}

Result<ReadWindowOptions::Response> ReadWindowOptions::execute(Server &server, Request request) {
    return server.window_options(request.target);
}

protocol::native::OperationReply ReadWindowOptions::write(Response response) {
    protocol::native::NativePayloadWriter writer;
    writer.string(response.server_instance);
    writer.u16(response.window ? 1 : 0);
    if (response.window) {
        writer.u64(response.window->value);
    }
    writer.u32(static_cast<std::uint32_t>(response.mode_keys));
    writer.u64(response.revision);
    return std::move(writer.bytes);
}
} // namespace tmux_cxx
