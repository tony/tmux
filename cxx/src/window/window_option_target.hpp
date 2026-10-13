#pragma once

#include "protocol/native/framing.hpp"

#include <tmux_cxx/window/operations/set_window_option.hpp>

#include <stdexcept>

namespace tmux_cxx {
/** @brief Decode the closed global-or-window target shared by window option requests. */
inline WindowOptionTarget read_window_option_target(protocol::native::NativePayloadReader &reader) {
    switch (reader.u16()) {
    case 0:
        return GlobalWindowOptions{};
    case 1:
        return WindowId{reader.u64()};
    default:
        throw std::invalid_argument("invalid window-option target tag");
    }
}
} // namespace tmux_cxx
