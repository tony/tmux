#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/read_window_options.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<WindowOptionsSnapshot> ServerConnection::window_options(WindowOptionTarget target) {
    const auto *window = std::get_if<WindowId>(&target);
    if (window != nullptr && window->value == 0) {
        return std::unexpected(channel_->operation_error(ReadWindowOptions::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "window identity must be nonzero"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u16(window != nullptr ? 1 : 0);
    if (window != nullptr) {
        arguments.u64(window->value);
    }
    auto reply = channel_->invoke_operation(ReadWindowOptions::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        auto owner = reader.string();
        const auto present = reader.u16();
        if (present > 1) {
            throw std::invalid_argument("invalid window-options target flag");
        }
        std::optional<WindowId> observed_window;
        if (present != 0) {
            observed_window = WindowId{reader.u64()};
        }
        const auto mode_keys = static_cast<WindowModeKeys>(reader.u32());
        const auto revision = reader.u64();
        reader.require_end();
        const auto requested_window = window != nullptr ? std::optional{*window} : std::nullopt;
        if (owner != channel_->observed_server_instance() || observed_window != requested_window ||
            !valid_window_mode_keys(mode_keys) ||
            (observed_window && (observed_window->value == 0 || revision == 0))) {
            throw std::invalid_argument("invalid window-options ownership, mode, or revision");
        }
        return WindowOptionsSnapshot{std::move(owner), observed_window, mode_keys, revision};
    } catch (const std::invalid_argument &error) {
        return std::unexpected(channel_->operation_error(ReadWindowOptions::description,
                                                         TmuxErrorCode::protocol, error.what()));
    } catch (const std::length_error &error) {
        return std::unexpected(channel_->operation_error(ReadWindowOptions::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
