// Header layouts translated from tmux 3.4 and 3.7; see docs/attribution.md.
/*
 * Copyright (c) 2023 Claudio Jeker <claudio@openbsd.org>
 * Copyright (c) 2003, 2004 Henning Brauer <henning@openbsd.org>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
#pragma once

#include <tmux_cxx/protocol/tmux/message.hpp>

#include <array>
#include <bit>
#include <cstddef>
#include <cstdint>
#include <span>
#include <string_view>
#include <sys/types.h>

/** @brief Stock tmux framing, compatibility evidence, and adapters with separate connection roles.
 */
namespace tmux_cxx::protocol::tmux {
/** @brief Distinguish stock imsg layouts that share the same protocol integer. */
enum class ImsgHeaderLayout { length16_flags16, length32_fd_marker };
/** @brief Declare the host payload ABI required by copied stock tmux layouts. */
struct TmuxPayloadAbi {
    std::endian byte_order;          ///< Host order used for integral payload fields.
    std::uint8_t int_bytes;          ///< Width of stock int and unsigned int payload fields.
    std::uint8_t long_bytes;         ///< Width of stock long and unsigned long payload fields.
    std::uint8_t pid_bytes;          ///< Width of the process identity carried in the imsg header.
    bool descriptors_use_scm_rights; ///< Descriptor ownership travels through SCM_RIGHTS.
};
/** @brief Name the client identity value encoded by one identification step. */
enum class TmuxIdentificationValue {
    client_flags,
    terminal_type,
    terminal_features,
    tty_name,
    working_directory,
    terminal_capabilities,
    standard_input,
    standard_output,
    process_id,
    environment,
    completion,
};
/** @brief Name control-related stock client flags carried during identification. */
enum class StockClientFlag : std::uint64_t {
    control_mode = 0x2000,
    control_mode_echo_disabled = 0x4000,
    control_mode_no_output = 0x4000000,
    control_mode_pause_after = 0x100000000,
    control_mode_wait_exit = 0x200000000,
};
/** @brief Test one named stock client flag without exposing bit arithmetic to adapter policy. */
constexpr bool has_stock_client_flag(std::uint64_t flags, StockClientFlag flag) noexcept {
    return (flags & static_cast<std::uint64_t>(flag)) != 0;
}
/** @brief Bind one ordered identification message to its client-owned value source. */
struct TmuxIdentificationStep {
    TmuxMessage message;           ///< Message emitted at this sequence position.
    TmuxIdentificationValue value; ///< Identity value encoded by this step.
};
/** @brief Declare the messages and traffic bound that complete stock client identification. */
struct TmuxIdentificationProfile {
    std::span<const TmuxMessage> messages; ///< Message identities admitted before completion.
    std::span<const TmuxIdentificationStep> sequence; ///< Ordered sender sequence for this profile.
    TmuxMessage completion; ///< Message that freezes identification and its selected profile.
    std::size_t max_bytes;  ///< Maximum identification traffic observed before rejection.
};
/** @brief Declare stock control-mode guards independently of whether an adapter supports them. */
struct TmuxControlModeProfile {
    std::string_view grammar;     ///< Stable grammar identity for policy and conformance fixtures.
    std::string_view begin_guard; ///< Prefix opening one command-output block.
    std::string_view end_guard;   ///< Prefix completing a successful command-output block.
    std::string_view error_guard; ///< Prefix completing a failed command-output block.
    bool pause_continue_notifications; ///< Grammar carries pane pause and continue notifications.
};
/** @brief Name the codec family implementing a profile's framing and payload declarations. */
enum class TmuxCodecFamily { host_order_imsg_v8 };
/** @brief Declare a stock wire layout and host ABI independently of release support policy. */
struct TmuxProtocolProfile {
    std::string_view name;   ///< Stable profile identity, separate from a tmux release.
    ImsgHeaderLayout layout; ///< Length and descriptor-marker layout within the 16-byte header.
    std::uint32_t protocol_version;  ///< Masked remote protocol integer required by this profile.
    std::uint32_t version_mask;      ///< Bits used by stock tmux's mandatory version check.
    std::size_t header_bytes;        ///< Stock header size in bytes.
    std::size_t max_frame_bytes;     ///< Admitted total frame bytes, including the header.
    std::uint32_t descriptor_marker; ///< Descriptor flag in the second host-order header word.
    TmuxPayloadAbi payload_abi;      ///< Host widths, byte order, and descriptor-transfer contract.
    std::span<const TmuxMessageDescription> messages; ///< Declared message and payload catalog.
    TmuxIdentificationProfile identification;         ///< Bounded client identification sequence.
    TmuxControlModeProfile control_mode; ///< Declared control grammar, separate from support.
    TmuxCodecFamily codec; ///< Codec implementation family used by both adapter roles.
};
inline constexpr TmuxPayloadAbi linux_x86_64_payload_abi{
    std::endian::little, 4, 8, 4, true}; ///< ABI established from pinned Linux x86_64 fixtures.
inline constexpr std::array stock_v8_identification_messages{
    TmuxMessage::identify_flags,     TmuxMessage::identify_term,
    TmuxMessage::identify_ttyname,   TmuxMessage::identify_stdin,
    TmuxMessage::identify_environ,   TmuxMessage::identify_done,
    TmuxMessage::identify_clientpid, TmuxMessage::identify_cwd,
    TmuxMessage::identify_features,  TmuxMessage::identify_stdout,
    TmuxMessage::identify_longflags, TmuxMessage::identify_terminfo,
}; ///< Messages declared by the protocol-eight identification sequence.
inline constexpr std::array legacy_identification_sequence{
    TmuxIdentificationStep{TmuxMessage::identify_flags, TmuxIdentificationValue::client_flags},
    TmuxIdentificationStep{TmuxMessage::identify_longflags, TmuxIdentificationValue::client_flags},
    TmuxIdentificationStep{TmuxMessage::identify_term, TmuxIdentificationValue::terminal_type},
    TmuxIdentificationStep{TmuxMessage::identify_features,
                           TmuxIdentificationValue::terminal_features},
    TmuxIdentificationStep{TmuxMessage::identify_ttyname, TmuxIdentificationValue::tty_name},
    TmuxIdentificationStep{TmuxMessage::identify_cwd, TmuxIdentificationValue::working_directory},
    TmuxIdentificationStep{TmuxMessage::identify_terminfo,
                           TmuxIdentificationValue::terminal_capabilities},
    TmuxIdentificationStep{TmuxMessage::identify_stdin, TmuxIdentificationValue::standard_input},
    TmuxIdentificationStep{TmuxMessage::identify_stdout, TmuxIdentificationValue::standard_output},
    TmuxIdentificationStep{TmuxMessage::identify_clientpid, TmuxIdentificationValue::process_id},
    TmuxIdentificationStep{TmuxMessage::identify_environ, TmuxIdentificationValue::environment},
    TmuxIdentificationStep{TmuxMessage::identify_done, TmuxIdentificationValue::completion},
}; ///< Sender order used by the pinned tmux 3.4 client.
inline constexpr std::array modern_identification_sequence{
    TmuxIdentificationStep{TmuxMessage::identify_longflags, TmuxIdentificationValue::client_flags},
    TmuxIdentificationStep{TmuxMessage::identify_longflags, TmuxIdentificationValue::client_flags},
    TmuxIdentificationStep{TmuxMessage::identify_term, TmuxIdentificationValue::terminal_type},
    TmuxIdentificationStep{TmuxMessage::identify_features,
                           TmuxIdentificationValue::terminal_features},
    TmuxIdentificationStep{TmuxMessage::identify_ttyname, TmuxIdentificationValue::tty_name},
    TmuxIdentificationStep{TmuxMessage::identify_cwd, TmuxIdentificationValue::working_directory},
    TmuxIdentificationStep{TmuxMessage::identify_terminfo,
                           TmuxIdentificationValue::terminal_capabilities},
    TmuxIdentificationStep{TmuxMessage::identify_stdin, TmuxIdentificationValue::standard_input},
    TmuxIdentificationStep{TmuxMessage::identify_stdout, TmuxIdentificationValue::standard_output},
    TmuxIdentificationStep{TmuxMessage::identify_clientpid, TmuxIdentificationValue::process_id},
    TmuxIdentificationStep{TmuxMessage::identify_environ, TmuxIdentificationValue::environment},
    TmuxIdentificationStep{TmuxMessage::identify_done, TmuxIdentificationValue::completion},
}; ///< Sender order used by the pinned tmux 3.7 client.
inline constexpr TmuxControlModeProfile stock_v8_control_mode{"percent-guard-lines-v1", "%begin",
                                                              "%end", "%error", true};
///< Guard and flow-control grammar declared independently of adapter support.
inline constexpr TmuxProtocolProfile legacy_profile{
    .name = "imsg-16-linux-x86_64",
    .layout = ImsgHeaderLayout::length16_flags16,
    .protocol_version = 8,
    .version_mask = 0xff,
    .header_bytes = 16,
    .max_frame_bytes = 16384,
    .descriptor_marker = 0x10000,
    .payload_abi = linux_x86_64_payload_abi,
    .messages = stock_v8_messages,
    .identification = {stock_v8_identification_messages, legacy_identification_sequence,
                       TmuxMessage::identify_done, 65536},
    .control_mode = stock_v8_control_mode,
    .codec = TmuxCodecFamily::host_order_imsg_v8,
};
///< Layout established from the immutable tmux 3.4 source.
inline constexpr TmuxProtocolProfile modern_profile{
    .name = "imsg-32-linux-x86_64",
    .layout = ImsgHeaderLayout::length32_fd_marker,
    .protocol_version = 8,
    .version_mask = 0xff,
    .header_bytes = 16,
    .max_frame_bytes = 16384,
    .descriptor_marker = 0x80000000,
    .payload_abi = linux_x86_64_payload_abi,
    .messages = stock_v8_messages,
    .identification = {stock_v8_identification_messages, modern_identification_sequence,
                       TmuxMessage::identify_done, 65536},
    .control_mode = stock_v8_control_mode,
    .codec = TmuxCodecFamily::host_order_imsg_v8,
};
///< Layout established from the immutable tmux 3.7 source.
/** @brief Reject hosts outside the stock profiles' declared byte order and native-width ABI. */
constexpr bool profile_matches_host(const TmuxProtocolProfile &profile) {
#if defined(__linux__) && defined(__x86_64__)
    return std::endian::native == profile.payload_abi.byte_order &&
           sizeof(int) == profile.payload_abi.int_bytes &&
           sizeof(long) == profile.payload_abi.long_bytes &&
           sizeof(pid_t) == profile.payload_abi.pid_bytes &&
           profile.payload_abi.descriptors_use_scm_rights;
#else
    (void)profile;
    return false;
#endif
}
/** @brief Require the host ABI shared by every built-in stock protocol profile. */
constexpr bool profile_matches_host() {
    return profile_matches_host(legacy_profile) && profile_matches_host(modern_profile);
}
} // namespace tmux_cxx::protocol::tmux
