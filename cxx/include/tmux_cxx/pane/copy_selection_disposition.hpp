#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tmux_cxx {
/** @brief Select the pane's mode effect after copied bytes are successfully stored. */
enum class CopySelectionDisposition : std::uint32_t {
    keep_selection = 0,
    clear_selection = 1,
    cancel_mode = 2
};
/** @brief Pair a successful-copy disposition with its native-language spelling. */
struct CopySelectionDispositionDescription {
    CopySelectionDisposition disposition; ///< Typed post-storage selection or mode effect.
    std::string_view name;                ///< Stable public choice spelling.
};
inline constexpr std::array copy_selection_dispositions{
    CopySelectionDispositionDescription{CopySelectionDisposition::keep_selection, "keep_selection"},
    CopySelectionDispositionDescription{CopySelectionDisposition::clear_selection,
                                        "clear_selection"},
    CopySelectionDispositionDescription{CopySelectionDisposition::cancel_mode, "cancel_mode"},
}; ///< Immutable dispositions shared by validation and language adapters.
/** @brief Reject selection-disposition representations absent from the immutable catalog. */
constexpr bool valid_copy_selection_disposition(CopySelectionDisposition value) {
    for (const auto &choice : copy_selection_dispositions) {
        if (choice.disposition == value) {
            return true;
        }
    }
    return false;
}
} // namespace tmux_cxx
