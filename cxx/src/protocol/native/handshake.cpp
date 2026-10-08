#include "protocol/native/handshake.hpp"

#include <set>
#include <stdexcept>

namespace tmux_cxx::protocol::native {
Frame handshake_request(std::uint16_t requested_version) {
    NativePayloadWriter request;
    request.string("");
    request.u16(requested_version);
    return {handshake_description.id, std::move(request.bytes)};
}
Bytes encode_handshake(std::span<const OperationDescription> operation_descriptions) {
    NativePayloadWriter handshake_payload;
    handshake_payload.u16(protocol_version);
    handshake_payload.u32(max_payload);
    std::uint64_t capabilities = 0;
    for (const auto &operation : operation_descriptions) {
        capabilities |= operation.required_capabilities;
    }
    handshake_payload.u64(capabilities);
    handshake_payload.u32(static_cast<std::uint32_t>(operation_descriptions.size()));
    for (const auto &operation : operation_descriptions) {
        handshake_payload.u16(operation.id.value);
        handshake_payload.string(operation.name);
        handshake_payload.u16(static_cast<std::uint16_t>(operation.effect));
        handshake_payload.u64(operation.required_capabilities);
    }
    return std::move(handshake_payload.bytes);
}
Result<NativeHandshake> decode_handshake(std::span<const std::uint8_t> payload) {
    try {
        NativePayloadReader reader(payload);
        NativeHandshake handshake{reader.u16(), reader.u32(), reader.u64(), {}};
        if (handshake.version != protocol_version || handshake.max_payload < 128 ||
            handshake.max_payload > max_payload ||
            (handshake.capabilities & ~known_capabilities) != 0) {
            throw std::invalid_argument("unsupported native negotiation declaration");
        }
        const auto operation_count = reader.u32();
        if (operation_count == 0 || operation_count > 1024) {
            throw std::invalid_argument("invalid native operation count");
        }
        std::set<std::string> operation_names;
        std::uint16_t previous_operation_id = 0;
        for (std::uint32_t operation_index = 0; operation_index < operation_count;
             ++operation_index) {
            AdvertisedOperation operation{OperationId{reader.u16()}, reader.string(),
                                          static_cast<OperationEffect>(reader.u16()), reader.u64()};
            if (operation.id.value <= previous_operation_id || operation.name.empty() ||
                !operation_names.insert(operation.name).second ||
                operation.effect > OperationEffect::program_input ||
                (operation.required_capabilities & ~handshake.capabilities) != 0) {
                throw std::invalid_argument("invalid native operation declaration");
            }
            previous_operation_id = operation.id.value;
            handshake.operations.push_back(std::move(operation));
        }
        reader.require_end();
        return handshake;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what(),
                                         std::string(handshake_description.name)});
    }
}
} // namespace tmux_cxx::protocol::native
