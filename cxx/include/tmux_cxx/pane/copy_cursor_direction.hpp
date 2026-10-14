#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tmux_cxx {
/** @brief Select counted relative movement of the copy-mode cursor. */
enum class CopyCursorDirection : std::uint32_t { left = 0, right = 1, up = 2, down = 3 };
/** @brief Pair a counted cursor direction with its native-language spelling. */
struct CopyCursorDirectionDescription {
    CopyCursorDirection direction; ///< Typed relative cursor direction.
    std::string_view name;         ///< Stable public choice spelling.
};
inline constexpr std::array copy_cursor_directions{
    CopyCursorDirectionDescription{CopyCursorDirection::left, "left"},
    CopyCursorDirectionDescription{CopyCursorDirection::right, "right"},
    CopyCursorDirectionDescription{CopyCursorDirection::up, "up"},
    CopyCursorDirectionDescription{CopyCursorDirection::down, "down"},
}; ///< Immutable directions shared by validation and language adapters.
/** @brief Reject cursor-direction representations absent from the immutable catalog. */
constexpr bool valid_copy_cursor_direction(CopyCursorDirection value) {
    for (const auto &choice : copy_cursor_directions) {
        if (choice.direction == value) {
            return true;
        }
    }
    return false;
}
} // namespace tmux_cxx
