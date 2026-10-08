#include "protocol/tmux/imsg_codec.hpp"
#include "protocol/tmux/payload_codec.hpp"
#include "protocol/tmux/stock_command_exchange.hpp"
#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>
#include <tmux_cxx/protocol/tmux/stock_profile_detector.hpp>

#include <gtest/gtest.h>

namespace tmux_cxx::protocol::tmux {
/** @brief Descriptor markers distinguish imsg layouts even when their protocol versions match. */
TEST(TmuxProtocol, DetectionUsesHeaderEvidenceRatherThanReleaseGuessing) {
    const auto ambiguous_command_frame = encode_imsg(message_number(TmuxMessage::command));
    StockProfileDetector detector;
    EXPECT_EQ(detector.observe(std::span(ambiguous_command_frame).first(8)).status,
              StockProfileDetectionStatus::need_more);
    EXPECT_EQ(detector.observe(ambiguous_command_frame).status,
              StockProfileDetectionStatus::ambiguous);
    const auto modern = encode_imsg(message_number(TmuxMessage::identify_stdin), {}, modern_profile,
                                    DescriptorTransfer::follows);
    auto selected = detector.observe(modern);
    ASSERT_EQ(selected.status, StockProfileDetectionStatus::matched);
    ASSERT_TRUE(selected.profile);
    EXPECT_EQ(selected.profile->layout, ImsgHeaderLayout::length32_fd_marker);
    const auto legacy = encode_imsg(message_number(TmuxMessage::identify_stdin), {}, legacy_profile,
                                    DescriptorTransfer::follows);
    EXPECT_EQ(detector.observe(legacy).status, StockProfileDetectionStatus::unknown);
}
/** @brief Both declared codecs preserve descriptor markers and mask protocol version evidence. */
TEST(TmuxProtocol, DecodePreservesDescriptorTransferAndMaskedVersion) {
    for (const auto &profile : {legacy_profile, modern_profile}) {
        auto bytes = encode_imsg(message_number(TmuxMessage::identify_stdin), {}, profile,
                                 DescriptorTransfer::follows);
        bytes[9] = 1;
        auto decoded = decode_imsg_header(bytes);
        ASSERT_TRUE(decoded);
        EXPECT_TRUE(decoded->has_descriptor);
        EXPECT_EQ(decoded->version & profile.version_mask, profile.protocol_version);
        EXPECT_EQ(decoded->length, profile.header_bytes);
        StockProfileDetector detector;
        auto selected = detector.observe(bytes);
        ASSERT_EQ(selected.status, StockProfileDetectionStatus::matched);
        EXPECT_EQ(selected.profile->layout, profile.layout);
    }
}
/** @brief The tmux payload codec follows the selected host ABI and rejects incompatible profiles.
 */
TEST(TmuxProtocol, PayloadCodecUsesDeclaredHostAbi) {
    HostPayloadWriter writer(legacy_profile);
    writer.write_u32(0x01020304);
    writer.write_u64(0x0102030405060708);
    EXPECT_EQ(writer.bytes(),
              (Bytes{0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01}));

    HostPayloadReader reader(legacy_profile, writer.bytes());
    EXPECT_EQ(reader.read_u32(), 0x01020304U);
    EXPECT_EQ(reader.read_u64(), 0x0102030405060708ULL);
    EXPECT_TRUE(reader.finished());

    auto incompatible = legacy_profile;
    incompatible.payload_abi.byte_order = std::endian::big;
    EXPECT_THROW((HostPayloadWriter{incompatible}), std::invalid_argument);
}
/** @brief A stock command exchange emits identification and assembles command streams. */
TEST(TmuxProtocol, StockCommandExchangeOwnsOneCommandState) {
    StockCommandExchange command_exchange(modern_profile);
    StockClientIdentification identification;
    identification.terminal_type = "dumb";
    identification.working_directory = "/";
    identification.process_id = 42;
    const std::vector<std::string> arguments{"display-message", "-p", "#{version}"};
    const auto request = command_exchange.begin_command(identification, arguments);
    ASSERT_TRUE(request);
    ASSERT_GE(request->size(), 3U);

    const auto first = decode_imsg_header(request->at(0).bytes);
    const auto second = decode_imsg_header(request->at(1).bytes);
    ASSERT_TRUE(first);
    ASSERT_TRUE(second);
    EXPECT_EQ(first->type, message_number(TmuxMessage::identify_longflags));
    EXPECT_EQ(second->type, message_number(TmuxMessage::identify_longflags));
    EXPECT_EQ(request->at(request->size() - 4).descriptor, StockClientDescriptor::standard_output);

    const auto command_header = decode_imsg_header(request->back().bytes);
    ASSERT_TRUE(command_header);
    EXPECT_EQ(command_header->type, message_number(TmuxMessage::command));
    const auto decoded_arguments = decode_command_arguments(
        modern_profile,
        std::span(request->back().bytes)
            .subspan(imsg_header_bytes, command_header->length - imsg_header_bytes));
    ASSERT_TRUE(decoded_arguments);
    EXPECT_EQ(*decoded_arguments, arguments);

    HostPayloadWriter open_payload(modern_profile);
    open_payload.write_u32(1);
    open_payload.write_u32(1);
    open_payload.write_u32(0);
    const auto opened = command_exchange.receive_server_traffic(
        encode_imsg(message_number(TmuxMessage::write_open), open_payload.bytes(), modern_profile));
    ASSERT_TRUE(opened);
    ASSERT_EQ(opened->client_replies.size(), 1U);
    const auto ready_header = decode_imsg_header(opened->client_replies.front().bytes);
    ASSERT_TRUE(ready_header);
    EXPECT_EQ(ready_header->type, message_number(TmuxMessage::write_ready));

    HostPayloadWriter data_payload(modern_profile);
    data_payload.write_u32(1);
    data_payload.write_bytes("3.7\n");
    HostPayloadWriter exit_payload(modern_profile);
    exit_payload.write_u32(0);
    Bytes response =
        encode_imsg(message_number(TmuxMessage::write_data), data_payload.bytes(), modern_profile);
    const auto exited =
        encode_imsg(message_number(TmuxMessage::exit), exit_payload.bytes(), modern_profile);
    response.insert(response.end(), exited.begin(), exited.end());

    const auto completed = command_exchange.receive_server_traffic(response);
    ASSERT_TRUE(completed);
    ASSERT_TRUE(completed->completed_command);
    EXPECT_EQ(completed->completed_command->exit_status, 0);
    EXPECT_EQ(completed->completed_command->standard_output, (Bytes{'3', '.', '7', '\n'}));
    EXPECT_TRUE(completed->completed_command->standard_error.empty());
}
/** @brief Profiles declare host ABI, messages, identification, grammar, and shared codec family. */
TEST(TmuxProtocol, ProfilesCarryCompleteWireDeclarations) {
    for (const auto &profile : {legacy_profile, modern_profile}) {
        EXPECT_TRUE(profile_matches_host(profile));
        EXPECT_EQ(profile.payload_abi.int_bytes, 4);
        EXPECT_EQ(profile.identification.completion, TmuxMessage::identify_done);
        EXPECT_EQ(profile.identification.max_bytes, 65536U);
        ASSERT_FALSE(profile.identification.sequence.empty());
        EXPECT_EQ(profile.identification.sequence.back().value,
                  TmuxIdentificationValue::completion);
        EXPECT_EQ(profile.control_mode.begin_guard, "%begin");
        EXPECT_TRUE(profile.control_mode.pause_continue_notifications);
        EXPECT_EQ(profile.codec, TmuxCodecFamily::host_order_imsg_v8);
        const auto *stdin_message =
            message_description(profile.messages, message_number(TmuxMessage::identify_stdin));
        ASSERT_NE(stdin_message, nullptr);
        ASSERT_TRUE(stdin_message->from_client);
        EXPECT_FALSE(stdin_message->from_server);
        EXPECT_EQ(stdin_message->from_client->descriptor, TmuxDescriptorRule::required);
        EXPECT_EQ(stdin_message->from_client->payload, TmuxPayloadLayout::empty);

        const auto *shell_message =
            message_description(profile.messages, message_number(TmuxMessage::shell));
        ASSERT_NE(shell_message, nullptr);
        ASSERT_TRUE(shell_message->from_client);
        ASSERT_TRUE(shell_message->from_server);
        EXPECT_EQ(shell_message->from_client->payload, TmuxPayloadLayout::empty);
        EXPECT_EQ(shell_message->from_server->payload, TmuxPayloadLayout::nul_terminated_bytes);

        EXPECT_NE(message_description(profile.messages, message_number(TmuxMessage::exec)),
                  nullptr);
        EXPECT_NE(message_description(profile.messages, message_number(TmuxMessage::flags)),
                  nullptr);
        EXPECT_NE(message_description(profile.messages, message_number(TmuxMessage::read_cancel)),
                  nullptr);
        EXPECT_NE(message_description(profile.messages, message_number(TmuxMessage::write_done)),
                  nullptr);
    }

    EXPECT_EQ(legacy_profile.identification.sequence.front().message, TmuxMessage::identify_flags);
    EXPECT_EQ(modern_profile.identification.sequence[0].message, TmuxMessage::identify_longflags);
    EXPECT_EQ(modern_profile.identification.sequence[1].message, TmuxMessage::identify_longflags);

    auto malformed = encode_imsg(message_number(TmuxMessage::identify_term), {'x'});
    const auto header = decode_imsg_header(malformed);
    ASSERT_TRUE(header);
    EXPECT_FALSE(validate_client_message(
        legacy_profile, *header,
        std::span(malformed).subspan(imsg_header_bytes, malformed.size() - imsg_header_bytes)));
    auto valid = encode_imsg(message_number(TmuxMessage::identify_term), {'x', 0});
    const auto valid_header = decode_imsg_header(valid);
    ASSERT_TRUE(valid_header);
    EXPECT_TRUE(validate_client_message(
        legacy_profile, *valid_header,
        std::span(valid).subspan(imsg_header_bytes, valid.size() - imsg_header_bytes)));
}
/** @brief Reserved flags, invalid lengths, and unsupported versions cannot establish a profile. */
TEST(TmuxProtocol, MalformedTrafficIsRejected) {
    auto bytes = encode_imsg(message_number(TmuxMessage::identify_stdin));
    bytes[6] = 2;
    EXPECT_FALSE(decode_imsg_header(bytes));
    StockProfileDetector detector;
    EXPECT_EQ(detector.observe(bytes).status, StockProfileDetectionStatus::unknown);
    bytes = encode_imsg(message_number(TmuxMessage::identify_stdin));
    bytes[8] = 255;
    EXPECT_EQ(detector.observe(bytes).status, StockProfileDetectionStatus::unknown);
}

/** @brief Policy reports tested interactive limits without inferring a release from framing.
 */
TEST(TmuxProtocol, CompatibilitySeparatesEvidenceFromReleaseClaims) {
    const auto report = TmuxCompatibilityPolicy{}.assess(
        legacy_profile, ProtocolDirection::stock_client_to_cxx_server, ConnectionMode::interactive);
    EXPECT_TRUE(report.release.empty());
    EXPECT_TRUE(report.release_uncertain);
    EXPECT_EQ(report.tier, SupportTier::general);
    EXPECT_EQ(report.support, CapabilitySupport::limited);
    EXPECT_EQ(report.verification, CompatibilityVerification::tested_subset);
    EXPECT_TRUE(report.quirks.empty());
    EXPECT_FALSE(report.evidence.empty());
    EXPECT_TRUE(std::ranges::contains(
        report.limitations,
        "ordinary-client borders use basic ASCII without capability-sensitive glyphs or styles"));
    EXPECT_TRUE(std::ranges::contains(
        report.limitations,
        "prefix and prefix2 accept None, one ASCII byte, or C-a through C-z; broader tmux key "
        "syntax is unsupported"));
    EXPECT_TRUE(std::ranges::contains(
        report.limitations,
        "set-option supports only -g and exact -t session assignments for prefix and prefix2; "
        "unset and implicit current-session assignment are unsupported"));
    EXPECT_TRUE(std::ranges::contains(
        report.limitations,
        "the prefix table contains only C-b send-prefix, quote and percent split, d detach, and o "
        "next-pane; configurable key tables and bindings are unsupported"));
    EXPECT_TRUE(std::ranges::contains(
        report.limitations,
        "paste-buffer requires -b name and -t %pane; delete-after, custom separators, raw "
        "paste, and bracketed paste are unsupported"));

    const auto control = TmuxCompatibilityPolicy{}.assess(
        legacy_profile, ProtocolDirection::stock_client_to_cxx_server, ConnectionMode::control);
    EXPECT_TRUE(std::ranges::contains(control.quirks, "unattached_control_channel_stays_open"));
    EXPECT_TRUE(std::ranges::contains(
        control.limitations,
        "an unattached control client may submit commands after its initial command"));
    EXPECT_TRUE(std::ranges::contains(control.limitations,
                                      "echo-disabled -CC control transport is unsupported"));
    EXPECT_TRUE(std::ranges::contains(
        control.limitations,
        "no-output, pause-after, and wait-exit control flags are rejected before effects"));
    EXPECT_TRUE(std::ranges::contains(
        control.evidence,
        "attached control clients receive session selection, committed session catalog and rename "
        "notifications, and new linked-pane output"));
    EXPECT_TRUE(std::ranges::contains(
        control.limitations,
        "pane output preserves per-pane byte order without reconstructing cross-pane chronology"));
    EXPECT_FALSE(std::ranges::contains(
        control.limitations, "control-client session attachment and switching are unsupported"));
    EXPECT_TRUE(std::ranges::contains(
        control.limitations,
        "a metadata observation gap closes the control client instead of resynchronizing it"));
    EXPECT_TRUE(std::ranges::contains(
        control.limitations, "control metadata retains at most 4096 committed session events"));

    const auto reverse = TmuxCompatibilityPolicy{}.assess(
        modern_profile, ProtocolDirection::cxx_client_to_stock_server, ConnectionMode::command);
    EXPECT_EQ(reverse.tier, SupportTier::unsupported);
    EXPECT_EQ(reverse.verification, CompatibilityVerification::unverified);

    const auto verified_reverse = TmuxCompatibilityPolicy{}.assess_stock_server(
        modern_profile, "3.7", ConnectionMode::command);
    EXPECT_EQ(verified_reverse.release, "3.7");
    EXPECT_FALSE(verified_reverse.release_uncertain);
    EXPECT_EQ(verified_reverse.tier, SupportTier::general);
    EXPECT_EQ(verified_reverse.support, CapabilitySupport::limited);
    EXPECT_EQ(verified_reverse.verification, CompatibilityVerification::tested_subset);
    EXPECT_TRUE(verified_reverse.commands.empty());
}
} // namespace tmux_cxx::protocol::tmux
