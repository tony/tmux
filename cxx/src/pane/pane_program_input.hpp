#pragma once

#include <tmux_cxx/tmux_error.hpp>

#include <string_view>

namespace tmux_cxx {
/** @brief Retain ordered program input through nonblocking PTY writes within a 256-KiB queue bound.
 */
class PaneProgramInput {
  public:
    /** @brief Admit all bytes or return backpressure before effects; a prior PTY failure closes
     * admission. */
    Result<void> admit(std::string_view bytes);
    /** @brief Deliver queued bytes while consuming the remaining byte budget; failures retain
     * undelivered bytes and close admission. */
    Result<void> flush_to_pty(int pty_descriptor, std::size_t &remaining_bytes);
    /** @brief Close admission when the PTY ends while retaining any undelivered bytes. */
    void close_admission();
    /** @brief Report undelivered bytes requiring writable PTY readiness. */
    bool delivery_pending() const;
    /** @brief Report whether delivery remains possible after earlier PTY failures. */
    bool accepting_input() const;
    /** @brief Return remaining queue capacity without admitting bytes. */
    std::size_t remaining_capacity() const;

  private:
    std::string queued_bytes_;      ///< Owned ordered bytes awaiting delivery.
    std::size_t sent_offset_ = 0;   ///< Delivered prefix retained until queue compaction.
    bool admission_closed_ = false; ///< PTY failure permanently closes further admission.
};
} // namespace tmux_cxx
