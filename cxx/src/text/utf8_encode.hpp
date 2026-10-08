#pragma once

#include <string>

namespace tmux_cxx::text {
/** @brief Append one Unicode scalar as UTF-8, replacing surrogate or out-of-range values. */
inline void append_utf8(std::string &text, char32_t scalar) {
    if (scalar > 0x10ffff || (scalar >= 0xd800 && scalar <= 0xdfff)) {
        scalar = 0xfffd;
    }
    if (scalar < 0x80) {
        text.push_back(static_cast<char>(scalar));
    } else if (scalar < 0x800) {
        text.push_back(static_cast<char>(0xc0 | (scalar >> 6)));
        text.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    } else if (scalar < 0x10000) {
        text.push_back(static_cast<char>(0xe0 | (scalar >> 12)));
        text.push_back(static_cast<char>(0x80 | ((scalar >> 6) & 0x3f)));
        text.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    } else {
        text.push_back(static_cast<char>(0xf0 | (scalar >> 18)));
        text.push_back(static_cast<char>(0x80 | ((scalar >> 12) & 0x3f)));
        text.push_back(static_cast<char>(0x80 | ((scalar >> 6) & 0x3f)));
        text.push_back(static_cast<char>(0x80 | (scalar & 0x3f)));
    }
}
} // namespace tmux_cxx::text
