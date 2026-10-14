#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tmux_cxx {
/** @brief Select an uncounted line or history boundary for the copy-mode cursor. */
enum class CopyCursorDestination : std::uint32_t {
    line_start = 0,
    line_end = 1,
    history_start = 2,
    history_end = 3
};
/** @brief Pair a cursor boundary with its native-language spelling. */
struct CopyCursorDestinationDescription {
    CopyCursorDestination destination; ///< Typed line or history boundary.
    std::string_view name;             ///< Stable public choice spelling.
};
inline constexpr std::array copy_cursor_destinations{
    CopyCursorDestinationDescription{CopyCursorDestination::line_start, "line_start"},
    CopyCursorDestinationDescription{CopyCursorDestination::line_end, "line_end"},
    CopyCursorDestinationDescription{CopyCursorDestination::history_start, "history_start"},
    CopyCursorDestinationDescription{CopyCursorDestination::history_end, "history_end"},
}; ///< Immutable destinations shared by validation and language adapters.
/** @brief Reject cursor-destination representations absent from the immutable catalog. */
constexpr bool valid_copy_cursor_destination(CopyCursorDestination value) {
    for (const auto &choice : copy_cursor_destinations) {
        if (choice.destination == value) {
            return true;
        }
    }
    return false;
}
} // namespace tmux_cxx
