#pragma once

#include <compare>
#include <cstdint>

namespace tmux_cxx {
/** @brief Locate a cell in chronological frozen copy-mode backing, including retained history. */
struct CopyPosition {
    /** @brief Absolute zero-based backing row. */
    std::uint32_t row;
    /** @brief Zero-based cell column, including a permitted end-of-line position. */
    std::uint32_t column;
    /** @brief Order copy positions by backing row and cell column. */
    auto operator<=>(const CopyPosition &) const = default;
};
} // namespace tmux_cxx
