#pragma once

#include "text/utf8.hpp"

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>

namespace tmux_cxx {
inline constexpr std::size_t paste_buffer_byte_limit = 262144; ///< Maximum bytes in one buffer.
inline constexpr std::size_t paste_buffer_named_limit = 50;    ///< Maximum retained named buffers.
inline constexpr std::size_t paste_buffer_automatic_limit = 50;
///< Maximum automatic buffers; the oldest automatic entry is evicted at this bound.

/** @brief Reject empty, oversized, invalid UTF-8, or control-bearing paste-buffer names. */
inline bool valid_paste_buffer_name(std::string_view name) {
    return !name.empty() && name.size() <= 128 && name.find('\0') == name.npos &&
           text::valid_utf8(name) &&
           std::ranges::none_of(name,
                                /** @brief Reject control bytes in a validated name. */
                                [](unsigned char byte) { return byte < 0x20 || byte == 0x7f; });
}

/** @brief Own server-global paste bytes and their automatic or explicitly named classification. */
struct PasteBuffer {
    std::string name;       ///< Valid UTF-8 name used for explicit lookup.
    std::string bytes;      ///< Arbitrary owned bytes, including NUL and invalid UTF-8.
    bool automatic;         ///< Included in default paste selection and automatic eviction.
    std::uint64_t revision; ///< Server mutation revision that stored these bytes.
};
} // namespace tmux_cxx
