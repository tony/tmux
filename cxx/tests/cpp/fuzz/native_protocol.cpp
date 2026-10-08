#include "protocol/native/framing.hpp"
#include "protocol/native/handshake.hpp"
#include "protocol/native/reply_envelope.hpp"

#include <algorithm>
#include <cstdlib>

/** @brief Fuzz bounded native traffic and preserve valid frame and reply round trips.
 *
 * Unexpected exceptions abort this isolated test process. */
extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data, std::size_t size) noexcept try {
    namespace native = tmux_cxx::protocol::native;
    if (size > 4096) {
        return 0;
    }
    const std::span input(data, size);
    if (const auto decoded = native::decode(input)) {
        const auto encoded = native::frame(decoded->operation, decoded->payload);
        if (!std::ranges::equal(encoded, input)) {
            std::abort();
        }
    }
    if (const auto reply = native::decode_reply_envelope(input)) {
        const auto encoded =
            native::encode_reply_envelope(reply->server_instance, reply->operation_result);
        if (!std::ranges::equal(encoded, input)) {
            std::abort();
        }
    }
    if (const auto handshake = native::decode_handshake(input)) {
        std::vector<native::OperationDescription> descriptions;
        for (const auto &operation : handshake->operations) {
            descriptions.push_back(
                {operation.id, operation.name, operation.effect, operation.required_capabilities});
        }
        const auto copied = native::decode_handshake(native::encode_handshake(descriptions));
        if (!copied || copied->operations.size() != handshake->operations.size()) {
            std::abort();
        }
    }
    return 0;
} catch (...) {
    std::abort();
}
