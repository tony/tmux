#pragma once

#include <tmux_cxx/protocol/bytes.hpp>
#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>

#include <span>
#include <string_view>

namespace tmux_cxx::protocol::tmux {
/** @brief Encode integral tmux payload fields using one declared host ABI. */
class HostPayloadWriter {
  public:
    /** @brief Bind encoding to a profile compatible with this process host.
     * @throws std::invalid_argument When the profile ABI differs from this host.
     */
    explicit HostPayloadWriter(const TmuxProtocolProfile &profile);
    /** @brief Append one host-order 32-bit tmux field. */
    void write_u32(std::uint32_t value);
    /** @brief Append one host-order 64-bit tmux field. */
    void write_u64(std::uint64_t value);
    /** @brief Append uninterpreted payload bytes after the preceding typed fields. */
    void write_bytes(std::span<const std::uint8_t> bytes);
    /** @brief Append a byte-oriented string without adding length or terminator fields. */
    void write_bytes(std::string_view bytes);
    /** @brief Append one NUL-terminated byte string or reject embedded terminators. */
    void write_c_string(std::string_view value);
    /** @brief Borrow the encoded payload until this writer mutates or is destroyed. */
    const Bytes &bytes() const noexcept;

  private:
    Bytes bytes_; ///< Host-ABI payload assembled in wire order.
};

/** @brief Decode integral tmux payload fields using one declared host ABI. */
class HostPayloadReader {
  public:
    /** @brief Borrow payload bytes under a profile compatible with this process host.
     * @throws std::invalid_argument When the profile ABI differs from this host.
     */
    HostPayloadReader(const TmuxProtocolProfile &profile, std::span<const std::uint8_t> bytes);
    /** @brief Read one host-order 32-bit field or reject truncated input. */
    std::uint32_t read_u32();
    /** @brief Read one host-order 64-bit field or reject truncated input. */
    std::uint64_t read_u64();
    /** @brief Report whether every borrowed payload byte has been consumed. */
    bool finished() const noexcept;
    /** @brief Reject payload bytes remaining after the declared layout. */
    void require_end() const;

  private:
    /** @brief Borrow the next field bytes or reject truncated input. */
    std::span<const std::uint8_t> read_field(std::size_t width);

    std::span<const std::uint8_t> bytes_; ///< Borrowed payload valid for this reader's lifetime.
    std::size_t offset_ = 0;              ///< Number of payload bytes already consumed.
};
} // namespace tmux_cxx::protocol::tmux
