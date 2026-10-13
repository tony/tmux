#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/window/mode_keys.hpp>

#include <optional>
#include <string>

namespace tmux_cxx {
/** @brief Copy effective window options with their server owner and mutation revision. */
struct WindowOptionsSnapshot {
    /** @brief Server lifetime owning these options. */
    std::string server_instance;
    /** @brief Shared window identity, absent for server-wide window defaults. */
    std::optional<WindowId> window;
    /** @brief Effective emacs or vi copy-mode key convention. */
    WindowModeKeys mode_keys;
    /** @brief Window metadata revision, or current server metadata revision for global defaults. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
