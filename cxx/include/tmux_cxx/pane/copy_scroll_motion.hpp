#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tmux_cxx {
/** @brief Select counted line, half-page, or page movement within frozen copy-mode backing. */
enum class CopyScrollMotion : std::uint32_t {
    line_up = 0,
    line_down = 1,
    half_page_up = 2,
    half_page_down = 3,
    page_up = 4,
    page_down = 5
};
/** @brief Pair a scroll motion with its native-language spelling. */
struct CopyScrollMotionDescription {
    CopyScrollMotion motion; ///< Typed direction and viewport step.
    std::string_view name;   ///< Stable public choice spelling.
};
inline constexpr std::array copy_scroll_motions{
    CopyScrollMotionDescription{CopyScrollMotion::line_up, "line_up"},
    CopyScrollMotionDescription{CopyScrollMotion::line_down, "line_down"},
    CopyScrollMotionDescription{CopyScrollMotion::half_page_up, "half_page_up"},
    CopyScrollMotionDescription{CopyScrollMotion::half_page_down, "half_page_down"},
    CopyScrollMotionDescription{CopyScrollMotion::page_up, "page_up"},
    CopyScrollMotionDescription{CopyScrollMotion::page_down, "page_down"},
}; ///< Immutable scroll motions shared by validation and language adapters.
/** @brief Reject scroll-motion representations absent from the immutable catalog. */
constexpr bool valid_copy_scroll_motion(CopyScrollMotion value) {
    for (const auto &choice : copy_scroll_motions) {
        if (choice.motion == value) {
            return true;
        }
    }
    return false;
}
} // namespace tmux_cxx
