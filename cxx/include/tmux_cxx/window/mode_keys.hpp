#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tmux_cxx {
/** @brief Select the window's emacs or vi copy-mode key conventions. */
enum class WindowModeKeys : std::uint32_t { emacs = 0, vi = 1 };

/** @brief Pair one closed mode-key value with its exact tmux option spelling. */
struct WindowModeKeysDescription {
    WindowModeKeys mode_keys; ///< Typed copy-mode key convention.
    std::string_view name;    ///< Canonical tmux choice name.
};

inline constexpr std::array window_mode_keys{
    WindowModeKeysDescription{WindowModeKeys::emacs, "emacs"},
    WindowModeKeysDescription{WindowModeKeys::vi, "vi"},
}; ///< Immutable choices shared by native validation, stock parsing, and language adapters.

/** @brief Reject mode-key representations absent from the immutable choice catalog. */
constexpr bool valid_window_mode_keys(WindowModeKeys value) {
    for (const auto &choice : window_mode_keys) {
        if (choice.mode_keys == value) {
            return true;
        }
    }
    return false;
}
} // namespace tmux_cxx
