#include "protocol/tmux/payload_codec.hpp"

#include <array>
#include <cstring>
#include <stdexcept>

namespace tmux_cxx::protocol::tmux {
namespace {
/** @brief Reject profiles whose declared payload ABI differs from this process host. */
void require_host_payload_abi(const TmuxProtocolProfile &profile) {
    if (!profile_matches_host(profile)) {
        throw std::invalid_argument("tmux payload profile does not match the host ABI");
    }
}
} // namespace

HostPayloadWriter::HostPayloadWriter(const TmuxProtocolProfile &profile) {
    require_host_payload_abi(profile);
}

void HostPayloadWriter::write_u32(std::uint32_t value) {
    std::array<std::uint8_t, sizeof(value)> encoded{};
    std::memcpy(encoded.data(), &value, encoded.size());
    bytes_.insert(bytes_.end(), encoded.begin(), encoded.end());
}

void HostPayloadWriter::write_u64(std::uint64_t value) {
    std::array<std::uint8_t, sizeof(value)> encoded{};
    std::memcpy(encoded.data(), &value, encoded.size());
    bytes_.insert(bytes_.end(), encoded.begin(), encoded.end());
}

void HostPayloadWriter::write_bytes(std::span<const std::uint8_t> bytes) {
    bytes_.insert(bytes_.end(), bytes.begin(), bytes.end());
}

void HostPayloadWriter::write_bytes(std::string_view bytes) {
    bytes_.insert(bytes_.end(), bytes.begin(), bytes.end());
}

void HostPayloadWriter::write_c_string(std::string_view value) {
    if (value.find('\0') != std::string_view::npos) {
        throw std::invalid_argument("tmux byte string contains an embedded terminator");
    }
    write_bytes(value);
    bytes_.push_back(0);
}

const Bytes &HostPayloadWriter::bytes() const noexcept { return bytes_; }

HostPayloadReader::HostPayloadReader(const TmuxProtocolProfile &profile,
                                     std::span<const std::uint8_t> bytes)
    : bytes_(bytes) {
    require_host_payload_abi(profile);
}

std::span<const std::uint8_t> HostPayloadReader::read_field(std::size_t width) {
    if (width > bytes_.size() - offset_) {
        throw std::invalid_argument("truncated tmux host-ABI payload field");
    }
    const auto field = bytes_.subspan(offset_, width);
    offset_ += width;
    return field;
}

std::uint32_t HostPayloadReader::read_u32() {
    std::uint32_t value{};
    const auto field = read_field(sizeof(value));
    std::memcpy(&value, field.data(), field.size());
    return value;
}

std::uint64_t HostPayloadReader::read_u64() {
    std::uint64_t value{};
    const auto field = read_field(sizeof(value));
    std::memcpy(&value, field.data(), field.size());
    return value;
}

bool HostPayloadReader::finished() const noexcept { return offset_ == bytes_.size(); }

void HostPayloadReader::require_end() const {
    if (!finished()) {
        throw std::invalid_argument("excess tmux host-ABI payload bytes");
    }
}
} // namespace tmux_cxx::protocol::tmux
