#pragma once

#include <tmux_cxx/client/client_snapshot.hpp>
#include <tmux_cxx/pane/pane_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/session/session_snapshot.hpp>
#include <tmux_cxx/window/window_snapshot.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

#include <cstddef>
#include <span>

/** @namespace tmux_cxx::protocol
 * @brief Wire contracts and adapters independent of server entity behavior.
 */
/** @brief Bounded framing and registration for the C++ server's native API. */
namespace tmux_cxx::protocol::native {
constexpr std::size_t header_size = 12;      ///< Native framing prefix measured in bytes.
constexpr std::size_t max_payload = 1048576; ///< Maximum payload bytes admitted per native frame.

/** @brief Own bytes encoded using the native protocol's counted little-endian contract. */
class NativePayloadWriter {
  public:
    /** @brief Append a little-endian 16-bit native value. */
    void u16(std::uint16_t value);
    /** @brief Append a little-endian 32-bit native value. */
    void u32(std::uint32_t value);
    /** @brief Append a little-endian 64-bit native value. */
    void u64(std::uint64_t value);
    /** @brief Append counted bytes; throw length_error when they exceed native frame capacity. */
    void string(std::string_view value);
    /** @brief Encode a materialized session value in the native wire contract. */
    void session(const SessionSnapshot &snapshot);
    /** @brief Encode a session's materialized window membership. */
    void window_link(const WindowLinkSnapshot &snapshot);
    /** @brief Encode a materialized shared-window value. */
    void window(const WindowSnapshot &snapshot);
    /** @brief Encode one pane's owner-qualified layout geometry. */
    void pane(const PaneSnapshot &snapshot);
    /** @brief Encode copied client lifecycle and attachment state. */
    void client(const ClientSnapshot &snapshot);
    /** @brief Encode one stock client's selected profile, support, evidence, and limits. */
    void stock_compatibility(const protocol::tmux::StockConnectionCompatibility &compatibility);
    Bytes bytes; ///< Owned encoded native payload.
};

/** @brief Borrow native bytes and validate each read against the remaining input. */
class NativePayloadReader {
  public:
    /** @brief Borrow the input bytes for this reader's lifetime. */
    explicit NativePayloadReader(std::span<const std::uint8_t> bytes) : bytes_(bytes) {}
    /** @brief Read a 16-bit value or throw invalid_argument on truncation. */
    std::uint16_t u16();
    /** @brief Read a 32-bit value or throw invalid_argument on truncation. */
    std::uint32_t u32();
    /** @brief Read a 64-bit value or throw invalid_argument on truncation. */
    std::uint64_t u64();
    /** @brief Copy counted bytes or throw invalid_argument on truncation. */
    std::string string();
    /** @brief Decode a copied session value; reject invalid UTF-8 names. */
    SessionSnapshot session();
    /** @brief Decode a window membership without retaining input references. */
    WindowLinkSnapshot window_link();
    /** @brief Decode copied shared-window state; reject invalid UTF-8 names. */
    WindowSnapshot window();
    /** @brief Decode one pane's bounded owner-qualified layout geometry. */
    PaneSnapshot pane();
    /** @brief Decode copied client state; reject impossible lifecycle combinations. */
    ClientSnapshot client();
    /** @brief Decode a bounded stock compatibility report without inferring release evidence. */
    protocol::tmux::StockConnectionCompatibility stock_compatibility();
    /** @brief Report whether every input byte has been consumed. */
    bool finished() const { return offset_ == bytes_.size(); }
    /** @brief Reject trailing arguments before any operation effects. */
    void require_end() const;
    /** @brief Borrow unconsumed bytes while the original input remains alive. */
    std::span<const std::uint8_t> remaining() const { return bytes_.subspan(offset_); }

  private:
    /** @brief Decode a bounded little-endian value or reject truncation. */
    std::uint64_t integer(std::size_t width);
    std::span<const std::uint8_t> bytes_; ///< Borrowed input valid for the reader lifetime.
    std::size_t offset_ = 0;              ///< Consumed byte count within the borrowed input.
};

/** @brief Own one validated native request or reply payload. */
struct Frame {
    OperationId operation; ///< Registered wire identity carried by this frame.
    Bytes payload;         ///< Owned payload excluding the native frame header.
};
/** @brief Encode one bounded versioned native frame. */
Bytes frame(OperationId operation, std::span<const std::uint8_t> payload);
/** @brief Validate a native header and return its complete bounded frame size. */
Result<std::size_t> frame_size(std::span<const std::uint8_t> frame_prefix);
/** @brief Decode exactly one complete frame; reject truncation and trailing bytes. */
Result<Frame> decode(std::span<const std::uint8_t> encoded_frame);
} // namespace tmux_cxx::protocol::native
