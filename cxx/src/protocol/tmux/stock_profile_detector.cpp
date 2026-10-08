#include "protocol/tmux/imsg_codec.hpp"
#include <tmux_cxx/protocol/tmux/stock_profile_detector.hpp>

namespace tmux_cxx::protocol::tmux {
StockProfileDetection StockProfileDetector::observe(std::span<const std::uint8_t> frame_header) {
    if (frame_header.size() < imsg_header_bytes) {
        return {StockProfileDetectionStatus::need_more, {}};
    }
    auto decoded_header = decode_imsg_header(frame_header);
    if (!decoded_header || (decoded_header->version & legacy_profile.version_mask) !=
                               legacy_profile.protocol_version) {
        return {StockProfileDetectionStatus::unknown, {}};
    }
    if (decoded_header->layout) {
        if (selected_layout_ && selected_layout_ != decoded_header->layout) {
            return {StockProfileDetectionStatus::unknown, {}};
        }
        selected_layout_ = decoded_header->layout;
    }
    if (!selected_layout_) {
        observed_identification_bytes_ += decoded_header->length;
        return {observed_identification_bytes_ > legacy_profile.identification.max_bytes
                    ? StockProfileDetectionStatus::unknown
                    : StockProfileDetectionStatus::ambiguous,
                {}};
    }
    return {StockProfileDetectionStatus::matched,
            *selected_layout_ == legacy_profile.layout ? legacy_profile : modern_profile};
}
} // namespace tmux_cxx::protocol::tmux
