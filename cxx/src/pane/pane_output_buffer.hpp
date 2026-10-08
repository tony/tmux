#pragma once

#include <tmux_cxx/pane/pane_output_cursor.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>

namespace tmux_cxx {
/** @brief Retain a bounded raw pane-output suffix under absolute byte offsets. */
class PaneOutputBuffer {
  public:
    /** @brief Carry copied bytes and the cursor immediately after them. */
    struct Read {
        std::string bytes;         ///< Retained bytes beginning at the requested cursor.
        std::uint64_t next_offset; ///< Absolute cursor following the returned bytes.
    };

    /** @brief Set the retained suffix capacity in bytes. */
    explicit PaneOutputBuffer(std::size_t capacity = pane_output_retention_bytes)
        : capacity_(capacity) {}
    /** @brief Append bytes in stream order and discard only the oldest excess prefix. */
    void append(std::string_view bytes);
    /** @brief Copy bytes at or after one absolute cursor, rejecting discarded or future offsets. */
    Result<Read> read_from(std::uint64_t offset) const;
    /** @brief Return the absolute cursor immediately after every byte observed so far. */
    std::uint64_t tail_offset() const;
    /** @brief Report whether any raw pane-output prefix has been discarded. */
    bool prefix_discarded() const;
    /** @brief Borrow the retained suffix until the next append. */
    std::string_view retained_bytes() const;

  private:
    std::size_t capacity_;         ///< Maximum retained suffix bytes.
    std::uint64_t begin_offset_{}; ///< Absolute offset of the retained suffix's first byte.
    std::string bytes_;            ///< Raw retained suffix in stream order.
};
} // namespace tmux_cxx
