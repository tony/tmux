#include <algorithm>
#include <cstdint>
#include <limits>

/** @brief Saturate untrusted terminal arithmetic before libvterm applies screen geometry bounds. */
extern "C" int tmux_cxx_vterm_saturating_add(int value, int delta) noexcept {
    const auto sum = static_cast<std::int64_t>(value) + static_cast<std::int64_t>(delta);
    return static_cast<int>(std::clamp(sum,
                                       static_cast<std::int64_t>(std::numeric_limits<int>::min()),
                                       static_cast<std::int64_t>(std::numeric_limits<int>::max())));
}

/** @brief Append one validated OSC command digit while saturating values beyond int range.
 * @param value Accumulated command number, or a negative initial marker.
 * @param digit Decimal digit from the parser, from zero through nine.
 */
extern "C" int tmux_cxx_vterm_append_decimal_digit(int value, int digit) noexcept {
    const auto prefix = value < 0 ? 0 : static_cast<std::int64_t>(value);
    const auto result = prefix * 10 + digit;
    return static_cast<int>(
        std::min(result, static_cast<std::int64_t>(std::numeric_limits<int>::max())));
}
