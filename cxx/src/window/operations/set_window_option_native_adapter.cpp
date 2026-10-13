#include "connection/native_server_channel.hpp"
#include <tmux_cxx/window/operations/set_window_option.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<void> ServerConnection::set_window_option(WindowOptionTarget target, std::string name,
                                                 std::string value) try {
    const auto *window = std::get_if<WindowId>(&target);
    if (window != nullptr && window->value == 0) {
        return std::unexpected(channel_->operation_error(SetWindowOption::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "window identity must be nonzero"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u16(window != nullptr ? 1 : 0);
    if (window != nullptr) {
        arguments.u64(window->value);
    }
    arguments.string(name);
    arguments.string(value);
    auto reply = channel_->invoke_operation(SetWindowOption::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    if (!reply->empty()) {
        return std::unexpected(channel_->operation_error(SetWindowOption::description,
                                                         TmuxErrorCode::protocol,
                                                         "excess window-option reply bytes"));
    }
    return {};
} catch (const std::length_error &error) {
    return std::unexpected(channel_->operation_error(
        SetWindowOption::description, TmuxErrorCode::invalid_argument, error.what()));
}
} // namespace tmux_cxx
