#include "protocol/tmux/imsg_codec.hpp"
#include "protocol/tmux/payload_codec.hpp"

#include <array>
#include <cstring>
#include <iterator>
#include <stdexcept>
#include <unistd.h>

namespace tmux_cxx::protocol::tmux {
using protocol::Bytes;
namespace {
/** @brief Distinguish the sender contract selected from one message declaration. */
enum class SenderRole { client, server };

/** @brief Validate one message against the contract declared for its sender role. */
Result<void> validate_message(const TmuxProtocolProfile &profile, const StockFrameHeader &header,
                              std::span<const std::uint8_t> payload, SenderRole sender) {
    const auto *message_declaration = message_description(profile.messages, header.type);
    if (message_declaration == nullptr) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "stock message is not declared"});
    }
    const auto &sender_contract = sender == SenderRole::client ? message_declaration->from_client
                                                               : message_declaration->from_server;
    if (!sender_contract) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "stock message has the wrong sender role"});
    }
    const bool descriptor_required = sender_contract->descriptor == TmuxDescriptorRule::required;
    if (header.has_descriptor != descriptor_required) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock message violates its descriptor contract"});
    }
    bool payload_valid = false;
    switch (sender_contract->payload) {
    case TmuxPayloadLayout::empty:
        payload_valid = payload.empty();
        break;
    case TmuxPayloadLayout::host_u32:
        payload_valid = payload.size() == profile.payload_abi.int_bytes;
        break;
    case TmuxPayloadLayout::host_pid:
        payload_valid = payload.size() == profile.payload_abi.pid_bytes;
        break;
    case TmuxPayloadLayout::exit_status:
        payload_valid = payload.empty() || payload.size() >= profile.payload_abi.int_bytes;
        break;
    case TmuxPayloadLayout::host_u64:
        payload_valid = payload.size() == 8;
        break;
    case TmuxPayloadLayout::host_u32_pair:
        payload_valid = payload.size() == 2 * profile.payload_abi.int_bytes;
        break;
    case TmuxPayloadLayout::host_u32_triple:
        payload_valid = payload.size() == 3 * profile.payload_abi.int_bytes;
        break;
    case TmuxPayloadLayout::nul_terminated_bytes:
        payload_valid = !payload.empty() && payload.back() == 0;
        break;
    case TmuxPayloadLayout::command_arguments:
        payload_valid = decode_command_arguments(profile, payload).has_value();
        break;
    case TmuxPayloadLayout::stream_data:
        payload_valid = payload.size() >= profile.payload_abi.int_bytes;
        break;
    case TmuxPayloadLayout::host_u32_pair_with_optional_path:
        payload_valid = payload.size() == 2 * profile.payload_abi.int_bytes ||
                        (payload.size() > 2 * profile.payload_abi.int_bytes && payload.back() == 0);
        break;
    case TmuxPayloadLayout::host_u32_triple_with_optional_path:
        payload_valid = payload.size() == 3 * profile.payload_abi.int_bytes ||
                        (payload.size() > 3 * profile.payload_abi.int_bytes && payload.back() == 0);
        break;
    case TmuxPayloadLayout::two_nul_terminated_bytes: {
        const auto first_terminator = std::ranges::find(payload, 0);
        payload_valid = first_terminator != payload.end() &&
                        std::next(first_terminator) != payload.end() && payload.back() == 0;
        break;
    }
    case TmuxPayloadLayout::opaque_bytes:
        payload_valid = true;
        break;
    }
    if (!payload_valid) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock message violates its payload contract"});
    }
    return {};
}
} // namespace

Bytes encode_imsg(std::uint32_t message_type, const Bytes &payload,
                  const TmuxProtocolProfile &profile, DescriptorTransfer descriptor) {
    if (!profile_matches_host()) {
        throw std::invalid_argument("stock profile does not match the host ABI");
    }
    if (profile.header_bytes != imsg_header_bytes || profile.max_frame_bytes < imsg_header_bytes ||
        profile.max_frame_bytes > legacy_profile.max_frame_bytes) {
        throw std::invalid_argument("stock profile declares invalid framing bounds");
    }
    if (payload.size() > profile.max_frame_bytes - profile.header_bytes) {
        throw std::length_error("stock frame exceeds its declared capacity");
    }
    const auto frame_length = static_cast<std::uint32_t>(profile.header_bytes + payload.size());
    const std::array<std::uint32_t, 4> header_words{
        message_type,
        frame_length | (descriptor == DescriptorTransfer::follows ? profile.descriptor_marker : 0),
        profile.protocol_version, static_cast<std::uint32_t>(::getpid())};
    Bytes frame(imsg_header_bytes);
    std::memcpy(frame.data(), header_words.data(), imsg_header_bytes);
    frame.insert(frame.end(), payload.begin(), payload.end());
    return frame;
}
Result<StockFrameHeader> decode_imsg_header(std::span<const std::uint8_t> frame_prefix) {
    if (!profile_matches_host() || frame_prefix.size() < imsg_header_bytes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "unsupported or truncated stock header"});
    }
    std::array<std::uint32_t, 4> header_words{};
    std::memcpy(header_words.data(), frame_prefix.data(), imsg_header_bytes);
    const auto descriptor_bits = header_words[1] & 0xffff0000;
    std::optional<ImsgHeaderLayout> header_layout;
    if (descriptor_bits == legacy_profile.descriptor_marker) {
        header_layout = legacy_profile.layout;
    } else if (descriptor_bits == modern_profile.descriptor_marker) {
        header_layout = modern_profile.layout;
    } else if (descriptor_bits != 0) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "invalid stock length or descriptor flags"});
    }
    const auto frame_length = header_words[1] & 0xffff;
    if (frame_length < imsg_header_bytes || frame_length > legacy_profile.max_frame_bytes) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, "invalid stock frame length"});
    }
    return StockFrameHeader{header_words[0], frame_length,    descriptor_bits != 0,
                            header_words[2], header_words[3], header_layout};
}
Result<void> validate_client_message(const TmuxProtocolProfile &profile,
                                     const StockFrameHeader &header,
                                     std::span<const std::uint8_t> payload) {
    return validate_message(profile, header, payload, SenderRole::client);
}

Result<void> validate_server_message(const TmuxProtocolProfile &profile,
                                     const StockFrameHeader &header,
                                     std::span<const std::uint8_t> payload) {
    return validate_message(profile, header, payload, SenderRole::server);
}
Result<std::vector<std::string>> decode_command_arguments(const TmuxProtocolProfile &profile,
                                                          std::span<const std::uint8_t> payload) {
    try {
        if (payload.size() < profile.payload_abi.int_bytes) {
            return std::unexpected(TmuxError{TmuxErrorCode::protocol, "truncated stock command"});
        }
        HostPayloadReader argument_count_field(profile,
                                               payload.first(profile.payload_abi.int_bytes));
        const auto argument_count = argument_count_field.read_u32();
        if (argument_count > 256) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "too many stock command arguments"});
        }
        std::vector<std::string> arguments;
        std::size_t offset = profile.payload_abi.int_bytes;
        for (std::uint32_t argument_index = 0; argument_index < argument_count; ++argument_index) {
            const auto argument_start = offset;
            while (offset < payload.size() && payload[offset] != 0) {
                ++offset;
            }
            if (offset == payload.size()) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::protocol, "unterminated stock argument"});
            }
            arguments.emplace_back(reinterpret_cast<const char *>(payload.data() + argument_start),
                                   offset - argument_start);
            ++offset;
        }
        if (offset != payload.size()) {
            return std::unexpected(TmuxError{TmuxErrorCode::protocol, "excess stock arguments"});
        }
        return arguments;
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}

Result<Bytes> encode_command_arguments(const TmuxProtocolProfile &profile,
                                       std::span<const std::string> arguments) {
    try {
        if (arguments.size() > 256) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "too many stock command arguments"});
        }
        HostPayloadWriter payload(profile);
        payload.write_u32(static_cast<std::uint32_t>(arguments.size()));
        for (const auto &argument : arguments) {
            payload.write_c_string(argument);
        }
        if (payload.bytes().size() > profile.max_frame_bytes - profile.header_bytes) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "stock command exceeds frame capacity"});
        }
        return payload.bytes();
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}
} // namespace tmux_cxx::protocol::tmux
