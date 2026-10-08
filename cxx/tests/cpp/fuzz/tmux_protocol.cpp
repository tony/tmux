#include "protocol/tmux/control/command_line.hpp"
#include "protocol/tmux/imsg_codec.hpp"
#include "protocol/tmux/payload_codec.hpp"
#include <tmux_cxx/protocol/tmux/stock_profile_detector.hpp>

#include <algorithm>
#include <cstdlib>

/** @brief Fuzz bounded stock headers, profile evidence, and command arguments without executing
 * commands.
 *
 * Unexpected exceptions abort this isolated protocol test. */
extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data, std::size_t size) noexcept try {
    namespace stock = tmux_cxx::protocol::tmux;
    if (size > 4096) {
        return 0;
    }
    const std::span input(data, size);
    stock::StockProfileDetector detector;
    const auto detection = detector.observe(input);
    const auto header = stock::decode_imsg_header(input);
    if (detection.status == stock::StockProfileDetectionStatus::matched &&
        (!header || !detection.profile || !header->layout ||
         *header->layout != detection.profile->layout)) {
        std::abort();
    }
    if (header) {
        const auto profile = header->layout == stock::modern_profile.layout ? stock::modern_profile
                                                                            : stock::legacy_profile;
        const auto payload = input.subspan(stock::imsg_header_bytes);
        const stock::Bytes copied(payload.begin(), payload.end());
        const auto descriptor = header->has_descriptor ? stock::DescriptorTransfer::follows
                                                       : stock::DescriptorTransfer::none;
        const auto encoded = stock::encode_imsg(header->type, copied, profile, descriptor);
        const auto decoded = stock::decode_imsg_header(encoded);
        if (!decoded || decoded->type != header->type || decoded->length != encoded.size() ||
            decoded->has_descriptor != header->has_descriptor) {
            std::abort();
        }
    }
    if (const auto arguments = stock::decode_command_arguments(stock::legacy_profile, input)) {
        const auto encoded = stock::encode_command_arguments(stock::legacy_profile, *arguments);
        if (!encoded || !std::ranges::equal(*encoded, input)) {
            std::abort();
        }
    }
    (void)tmux_cxx::protocol::tmux::control::parse_command_line(
        std::string_view(reinterpret_cast<const char *>(data), size));
    return 0;
} catch (...) {
    std::abort();
}
