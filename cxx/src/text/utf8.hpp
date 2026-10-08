#pragma once

#include <cstdint>
#include <string_view>

/** @brief Validate shared names and wire text without changing binary program traffic. */
namespace tmux_cxx::text {
/** @brief Validate UTF-8 text fields while rejecting overlong, truncated, surrogate, and
 * out-of-range scalar encodings. */
inline bool valid_utf8(std::string_view text) {
    std::size_t position = 0;
    while (position < text.size()) {
        const auto first = static_cast<std::uint8_t>(text[position++]);
        if (first < 0x80) {
            continue;
        }
        std::size_t remaining;
        std::uint32_t scalar;
        std::uint32_t minimum;
        if (first >= 0xc2 && first <= 0xdf) {
            remaining = 1;
            scalar = first & 0x1f;
            minimum = 0x80;
        } else if (first >= 0xe0 && first <= 0xef) {
            remaining = 2;
            scalar = first & 0x0f;
            minimum = 0x800;
        } else if (first >= 0xf0 && first <= 0xf4) {
            remaining = 3;
            scalar = first & 0x07;
            minimum = 0x10000;
        } else {
            return false;
        }
        if (remaining > text.size() - position) {
            return false;
        }
        while (remaining-- != 0) {
            const auto continuation = static_cast<std::uint8_t>(text[position++]);
            if ((continuation & 0xc0) != 0x80) {
                return false;
            }
            scalar = (scalar << 6) | (continuation & 0x3f);
        }
        if (scalar < minimum || scalar > 0x10ffff || (scalar >= 0xd800 && scalar <= 0xdfff)) {
            return false;
        }
    }
    return true;
}
} // namespace tmux_cxx::text
