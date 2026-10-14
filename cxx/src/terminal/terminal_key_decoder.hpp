#pragma once

#include <tmux_cxx/key_table/key_code.hpp>

#include <chrono>
#include <cstddef>
#include <optional>
#include <string_view>

namespace tmux_cxx {
/** @brief Distinguish incomplete input, normalized keys, and unsupported raw terminal spans. */
enum class TerminalKeyStatus { pending, decoded, unsupported };

/** @brief Describe one front key while its original bytes remain owned by client input. */
struct DecodedTerminalKey {
    TerminalKeyStatus status;              ///< Pending results consume no bytes.
    std::optional<key_table::KeyCode> key; ///< Normalized identity only for decoded keys.
    std::size_t byte_count;                ///< Exact original span; positive for completed results.
};

/** @brief Retain one incomplete-key deadline and completed-key metadata through client
 * backpressure. */
class TerminalKeyDecoder {
  public:
    using Clock = std::chrono::steady_clock;             ///< Monotonic terminal key deadlines.
    static constexpr std::size_t maximum_key_bytes = 32; ///< Bound escape grammar inspection.

    /** @brief Inspect the pending front key without consuming its raw bytes.
     *
     * The caller may append bytes but must preserve the front span until consume().
     */
    DecodedTerminalKey peek(std::string_view bytes, Clock::time_point now);
    /** @brief Forget the front key after its complete raw span is consumed. */
    void consume();
    /** @brief Return the deadline of an incomplete terminal key. */
    std::optional<Clock::time_point> next_deadline() const;

  private:
    std::optional<DecodedTerminalKey> completed_; ///< Metadata only; no borrowed input lifetime.
    std::optional<Clock::time_point> deadline_; ///< First-fragment deadline, unaffected by appends.
};
} // namespace tmux_cxx
