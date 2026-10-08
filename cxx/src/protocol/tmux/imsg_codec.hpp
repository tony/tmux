// Protocol values and layouts translated from tmux 3.4's tmux-protocol.h
// and compat/imsg.h. See docs/attribution.md for immutable source revisions.
/*
 * Copyright (c) 2021 Nicholas Marriott <nicholas.marriott@gmail.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
 * IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
 * OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
#pragma once

#include <tmux_cxx/protocol/bytes.hpp>
#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <string>
#include <vector>

namespace tmux_cxx::protocol::tmux {
using protocol::Bytes;
constexpr std::size_t imsg_header_bytes =
    16; ///< Stock header bytes independent of C++ structure padding.
/** @brief Materialize stock message metadata after validating its wire layout and traffic bound. */
struct StockFrameHeader {
    std::uint32_t type;    ///< Stock message identity in the selected profile.
    std::uint32_t length;  ///< Total frame bytes, including the wire header.
    bool has_descriptor;   ///< This message owns the next transferred descriptor.
    std::uint32_t version; ///< Protocol integer; this alone does not identify a release.
    std::uint32_t pid;     ///< Sender PID field in the stock host ABI.
    std::optional<ImsgHeaderLayout> layout; ///< Descriptor evidence; absent for common framing.
};
/** @brief Legacy protocol integer; release evidence remains separate. */
constexpr std::uint32_t protocol_version = 8;

/** @brief Declare whether an encoded stock frame transfers one descriptor. */
enum class DescriptorTransfer { none, follows };

/** @brief Encode declared stock framing; length_error rejects excessive bytes and invalid_argument
 * rejects an unsupported host. */
Bytes encode_imsg(std::uint32_t message_type, const Bytes &payload = {},
                  const TmuxProtocolProfile &profile = legacy_profile,
                  DescriptorTransfer descriptor = DescriptorTransfer::none);
/** @brief Decode a bounded stock header and retain ambiguous layout evidence without guessing a
 * release. */
Result<StockFrameHeader> decode_imsg_header(std::span<const std::uint8_t> frame_prefix);
/** @brief Validate one stock-client message against its declared direction, payload, and descriptor
 * contract. */
Result<void> validate_client_message(const TmuxProtocolProfile &profile,
                                     const StockFrameHeader &header,
                                     std::span<const std::uint8_t> payload);
/** @brief Validate one stock-server message against its declared payload and descriptor contract.
 */
Result<void> validate_server_message(const TmuxProtocolProfile &profile,
                                     const StockFrameHeader &header,
                                     std::span<const std::uint8_t> payload);
/** @brief Decode bounded NUL-terminated argv entries without executing commands. */
Result<std::vector<std::string>> decode_command_arguments(const TmuxProtocolProfile &profile,
                                                          std::span<const std::uint8_t> payload);
/** @brief Encode bounded stock command arguments without parsing or executing them. */
Result<Bytes> encode_command_arguments(const TmuxProtocolProfile &profile,
                                       std::span<const std::string> arguments);
} // namespace tmux_cxx::protocol::tmux
