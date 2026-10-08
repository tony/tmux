#pragma once

#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>

#include <optional>
#include <span>

namespace tmux_cxx::protocol::tmux {
/** @brief Report whether stock framing evidence selects one declared wire profile. */
enum class StockProfileDetectionStatus { need_more, matched, ambiguous, unknown };
/** @brief Describe stock wire-profile selection without inferring a tmux release. */
struct StockProfileDetection {
    StockProfileDetectionStatus status; ///< Result of observing the current framing evidence.
    std::optional<TmuxProtocolProfile> profile; ///< Selected wire profile; release remains unknown.
};
/** @brief Bound stock profile detection to 64 KiB of identification traffic. */
class StockProfileDetector {
  public:
    /** @brief Start unresolved or use an explicitly chosen layout; contradictory traffic is
     * rejected. */
    explicit StockProfileDetector(std::optional<ImsgHeaderLayout> layout = {})
        : selected_layout_(layout) {}
    /** @brief Observe one complete message's header once; distinguish layouts using descriptor
     * markers. */
    StockProfileDetection observe(std::span<const std::uint8_t> frame_header);

  private:
    std::optional<ImsgHeaderLayout>
        selected_layout_; ///< Selected layout retained across message boundaries.
    std::size_t observed_identification_bytes_ =
        0; ///< Identification bytes observed before layout selection.
};
} // namespace tmux_cxx::protocol::tmux
