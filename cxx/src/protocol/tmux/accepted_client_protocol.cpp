#include "protocol/tmux/accepted_client_protocol.hpp"

#include "client/stock_client_lifecycle.hpp"
#include "commands/command_catalog.hpp"
#include "protocol/native/framing.hpp"
#include "protocol/tmux/control/command_line.hpp"
#include "protocol/tmux/control/limits.hpp"
#include "protocol/tmux/payload_codec.hpp"
#include "session/session_event_reader.hpp"
#include <tmux_cxx/client/operations/resize_client.hpp>
#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>
#include <tmux_cxx/protocol/tmux/protocol_quirk.hpp>
#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <stdexcept>
#include <unistd.h>
#include <utility>

namespace tmux_cxx::protocol::tmux {
namespace {
/** @brief Test whether one validated message belongs to the selected identification catalog. */
bool is_identification_message(const TmuxProtocolProfile &profile, TmuxMessage message) {
    return std::ranges::contains(profile.identification.messages, message);
}

/** @brief Invoke one admitted registered command under its declared compatibility policy. */
Result<commands::CommandOutcome> execute_registered_command(
    Server &server, ClientId client_id, const commands::CommandCatalog &command_catalog,
    const StockConnectionCompatibility &compatibility, std::span<const std::string> arguments) {
    if (compatibility.support == CapabilitySupport::unsupported) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "requested stock connection mode is unsupported"});
    }
    commands::CommandInvocation invocation{server, client_id};
    return command_catalog.invoke(invocation, arguments);
}

/** @brief Classify the supported client mode after rejecting known control variants. */
ConnectionMode admitted_connection_mode(std::uint64_t client_flags) {
    if (has_stock_client_flag(client_flags, StockClientFlag::control_mode_echo_disabled) ||
        has_stock_client_flag(client_flags, StockClientFlag::control_mode_no_output) ||
        has_stock_client_flag(client_flags, StockClientFlag::control_mode_pause_after) ||
        has_stock_client_flag(client_flags, StockClientFlag::control_mode_wait_exit)) {
        throw std::invalid_argument("unsupported stock control-mode flags");
    }
    return has_stock_client_flag(client_flags, StockClientFlag::control_mode)
               ? ConnectionMode::control
               : ConnectionMode::command;
}
} // namespace

void AcceptedClientProtocol::queue_server_frame(Bytes frame) {
    if (outbound_bytes_.size() + frame.size() > native::max_payload + 4096) {
        throw std::length_error("stock reply exceeds transport capacity");
    }
    outbound_bytes_.insert(outbound_bytes_.end(), frame.begin(), frame.end());
}

void AcceptedClientProtocol::queue_command_exit(int exit_status) {
    HostPayloadWriter status(require_selected_profile());
    status.write_u32(static_cast<std::uint32_t>(exit_status));
    queue_server_frame(
        encode_imsg(message_number(TmuxMessage::exit), status.bytes(), require_selected_profile()));
    phase_.emplace<ClosingTransportPhase>();
}

const TmuxProtocolProfile &AcceptedClientProtocol::validation_profile() const {
    if (const auto *identifying = std::get_if<IdentifyingPhase>(&phase_)) {
        return identifying->selected_profile ? *identifying->selected_profile : legacy_profile;
    }
    if (const auto *awaiting_command = std::get_if<AwaitingOneShotCommandPhase>(&phase_)) {
        return awaiting_command->admission.profile;
    }
    if (const auto *awaiting_control = std::get_if<AwaitingControlStartCommandPhase>(&phase_)) {
        return awaiting_control->admission.profile;
    }
    if (const auto *command_output = std::get_if<OneShotCommandOutputPhase>(&phase_)) {
        return command_output->admission.profile;
    }
    if (const auto *attached = std::get_if<AttachedTerminalPhase>(&phase_)) {
        return attached->admission.profile;
    }
    if (const auto *control_mode = std::get_if<ControlModePhase>(&phase_)) {
        return control_mode->admission.profile;
    }
    if (const auto *draining_control = std::get_if<DrainingControlModePhase>(&phase_)) {
        return draining_control->admission.profile;
    }
    if (const auto *detaching = std::get_if<DetachingTerminalPhase>(&phase_)) {
        return detaching->admission.profile;
    }
    throw std::logic_error("closing tmux client has no traffic to validate");
}

const TmuxProtocolProfile &AcceptedClientProtocol::require_selected_profile() const {
    if (const auto *identifying = std::get_if<IdentifyingPhase>(&phase_)) {
        if (identifying->selected_profile) {
            return *identifying->selected_profile;
        }
        throw std::logic_error("accepted tmux client has no selected protocol profile");
    }
    return validation_profile();
}

bool AcceptedClientProtocol::closes_after_outbound() const {
    return std::holds_alternative<ClosingTransportPhase>(phase_);
}

std::optional<AcceptedClientProtocol::Clock::time_point>
AcceptedClientProtocol::next_protocol_deadline() const {
    if (const auto *identifying = std::get_if<IdentifyingPhase>(&phase_)) {
        return identifying->deadline;
    }
    if (const auto *detaching = std::get_if<DetachingTerminalPhase>(&phase_)) {
        return detaching->deadline;
    }
    return std::nullopt;
}

ClientId AcceptedClientProtocol::client_id() const { return client_id_; }

std::optional<AcceptedClientProtocol::ControlModeRoute>
AcceptedClientProtocol::control_mode_route() const {
    if (const auto *control_mode = std::get_if<ControlModePhase>(&phase_)) {
        return ControlModeRoute{control_mode->channel->input_descriptor(),
                                control_mode->channel->output_descriptor(), true,
                                control_mode->channel->output_pending()};
    }
    if (const auto *draining_control = std::get_if<DrainingControlModePhase>(&phase_)) {
        return ControlModeRoute{draining_control->channel->input_descriptor(),
                                draining_control->channel->output_descriptor(), false,
                                draining_control->channel->output_pending()};
    }
    return std::nullopt;
}

Result<AcceptedClientProtocol::TransportAction>
AcceptedClientProtocol::advance_server_driven_output(Server &server,
                                                     const ClientSnapshot &client_snapshot) {
    try {
        if (auto *attached = std::get_if<AttachedTerminalPhase>(&phase_);
            attached && !client_snapshot.session) {
            if (client_snapshot.detached_from.empty()) {
                throw std::logic_error("detached client has no session name");
            }
            Bytes session_name(client_snapshot.detached_from.begin(),
                               client_snapshot.detached_from.end());
            session_name.push_back(0);
            queue_server_frame(encode_imsg(message_number(TmuxMessage::detach), session_name,
                                           attached->admission.profile));
            auto stock_admission = std::move(attached->admission);
            phase_.emplace<DetachingTerminalPhase>(std::move(stock_admission),
                                                   Clock::now() + std::chrono::milliseconds(250));
        }
        if (auto *control_mode = std::get_if<ControlModePhase>(&phase_)) {
            if (auto notifications_queued = control_mode->session_notifications.queue_notifications(
                    server, *control_mode->channel);
                !notifications_queued) {
                return std::unexpected(std::move(notifications_queued.error()));
            }
            if (auto pane_output_queued = control_mode->session_output.queue_new_output(
                    server, client_snapshot, *control_mode->channel);
                !pane_output_queued) {
                return std::unexpected(std::move(pane_output_queued.error()));
            }
        }
        return TransportAction{std::exchange(outbound_bytes_, {}), closes_after_outbound()};
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}

Result<AcceptedClientProtocol::TransportAction>
AcceptedClientProtocol::advance_control_mode(ControlModeReadiness readiness, Server &server,
                                             const commands::CommandCatalog &command_catalog,
                                             control::CommandSequence &control_sequence) {
    try {
        if (readiness.descriptor_failed || readiness.output_closed) {
            throw std::runtime_error("stock control descriptor route failed");
        }
        if (auto *control_mode = std::get_if<ControlModePhase>(&phase_);
            control_mode && (readiness.input_ready || readiness.input_closed)) {
            auto input_batch = control_mode->channel->read_input(control::read_bytes_per_turn);
            if (!input_batch) {
                throw std::runtime_error(input_batch.error().message);
            }
            for (const auto &command_line : input_batch->command_lines) {
                invoke_control_command(server, command_catalog, control_sequence, command_line);
            }
            if (input_batch->exit_requested || readiness.input_closed) {
                auto stock_admission = std::move(control_mode->admission);
                auto channel = std::move(control_mode->channel);
                phase_.emplace<DrainingControlModePhase>(std::move(stock_admission),
                                                         std::move(channel));
            }
        }
        if (readiness.output_ready) {
            control::Channel *channel = nullptr;
            if (auto *control_mode = std::get_if<ControlModePhase>(&phase_)) {
                channel = control_mode->channel.get();
            } else if (auto *draining_control = std::get_if<DrainingControlModePhase>(&phase_)) {
                channel = draining_control->channel.get();
            }
            if (channel) {
                std::size_t output_budget = control::write_bytes_per_turn;
                if (auto flushed = channel->flush_output(output_budget); !flushed) {
                    throw std::runtime_error(flushed.error().message);
                }
            }
        }
        if (const auto *draining_control = std::get_if<DrainingControlModePhase>(&phase_);
            draining_control && !draining_control->channel->output_pending()) {
            queue_command_exit(0);
        }
        return TransportAction{std::exchange(outbound_bytes_, {}), closes_after_outbound()};
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}

Result<AcceptedClientProtocol::TransportAction> AcceptedClientProtocol::receive_client_traffic(
    std::span<const std::uint8_t> client_bytes, std::vector<posix::OwnedFd> transferred_descriptors,
    Server &server, const commands::CommandCatalog &command_catalog,
    control::CommandSequence &control_sequence) {
    try {
        if (!transferred_descriptors.empty()) {
            auto *identifying = std::get_if<IdentifyingPhase>(&phase_);
            if (!identifying) {
                throw std::invalid_argument(
                    "stock descriptors are valid only during identification");
            }
            for (auto &descriptor : transferred_descriptors) {
                identifying->transferred_descriptors.push_back(std::move(descriptor));
            }
            if (identifying->transferred_descriptors.size() > 16) {
                throw std::length_error("stock identification exceeds descriptor capacity");
            }
        }
        if (inbound_bytes_.size() + client_bytes.size() >
            native::max_payload + native::header_size) {
            throw std::length_error("stock input exceeds transport capacity");
        }
        inbound_bytes_.insert(inbound_bytes_.end(), client_bytes.begin(), client_bytes.end());
        while (inbound_bytes_.size() >= imsg_header_bytes && !closes_after_outbound()) {
            auto decoded_header = decode_imsg_header(inbound_bytes_);
            if (!decoded_header) {
                return std::unexpected(decoded_header.error());
            }
            const auto &header = *decoded_header;
            if (inbound_bytes_.size() < header.length) {
                break;
            }
            if (auto *identifying = std::get_if<IdentifyingPhase>(&phase_)) {
                auto profile_detection = identifying->profile_detector.observe(inbound_bytes_);
                if (profile_detection.status == StockProfileDetectionStatus::unknown) {
                    queue_server_frame(encode_imsg(message_number(TmuxMessage::version)));
                    phase_.emplace<ClosingTransportPhase>();
                    break;
                }
                if (profile_detection.profile) {
                    identifying->selected_profile = profile_detection.profile;
                }
            }
            const auto payload = std::span(inbound_bytes_)
                                     .subspan(imsg_header_bytes, header.length - imsg_header_bytes);
            consume_client_message(server, command_catalog, header, payload, control_sequence);
            inbound_bytes_.erase(inbound_bytes_.begin(), inbound_bytes_.begin() + header.length);
        }
        return TransportAction{std::exchange(outbound_bytes_, {}), closes_after_outbound()};
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}

void AcceptedClientProtocol::adopt_identification_descriptor(TmuxMessage message) {
    auto *identifying = std::get_if<IdentifyingPhase>(&phase_);
    if (!identifying) {
        throw std::invalid_argument("stock descriptor arrived after identification");
    }
    if (identifying->transferred_descriptors.empty()) {
        throw std::invalid_argument("missing stock identification descriptor");
    }
    auto descriptor = std::move(identifying->transferred_descriptors.front());
    identifying->transferred_descriptors.pop_front();
    if (message == TmuxMessage::identify_stdin) {
        identifying->stdin_descriptor = std::move(descriptor);
    } else if (message == TmuxMessage::identify_stdout) {
        identifying->stdout_descriptor = std::move(descriptor);
    } else {
        throw std::invalid_argument("unsupported stock identification descriptor");
    }
}

void AcceptedClientProtocol::record_client_flags(TmuxMessage message,
                                                 std::span<const std::uint8_t> payload) {
    auto *identifying = std::get_if<IdentifyingPhase>(&phase_);
    if (!identifying) {
        throw std::invalid_argument("stock client flags arrived after identification");
    }
    const auto &wire_profile =
        identifying->selected_profile ? *identifying->selected_profile : legacy_profile;
    HostPayloadReader flag_payload(wire_profile, payload);
    const auto observed_flags =
        message == TmuxMessage::identify_flags ? flag_payload.read_u32() : flag_payload.read_u64();
    flag_payload.require_end();
    identifying->client_flags |= observed_flags;
}

void AcceptedClientProtocol::complete_identification(
    Server &server, const commands::CommandCatalog &command_catalog,
    std::span<const std::uint8_t> payload) {
    auto *identifying = std::get_if<IdentifyingPhase>(&phase_);
    if (!identifying || !identifying->selected_profile || !payload.empty()) {
        throw std::invalid_argument("stock identification has no selected profile");
    }
    const auto connection_mode = admitted_connection_mode(identifying->client_flags);
    auto compatibility = TmuxCompatibilityPolicy{}.assess(
        *identifying->selected_profile, ProtocolDirection::stock_client_to_cxx_server,
        connection_mode);
    compatibility.commands = command_catalog.spellings();
    auto selected_profile = *identifying->selected_profile;
    if (connection_mode == ConnectionMode::control) {
        auto control_channel = control::Channel::adopt(std::move(identifying->stdin_descriptor),
                                                       std::move(identifying->stdout_descriptor));
        if (!control_channel) {
            throw std::runtime_error(control_channel.error().message);
        }
        auto client_identification =
            StockClientLifecycle::identify_command(server, client_id_, compatibility);
        if (!client_identification) {
            throw std::runtime_error(client_identification.error().message);
        }
        phase_.emplace<AwaitingControlStartCommandPhase>(
            StockClientAdmission{std::move(selected_profile), std::move(compatibility)},
            std::move(*control_channel), SessionEventReader::tail(server));
        return;
    }
    Result<void> client_identification =
        identifying->stdin_descriptor.get() >= 0 && identifying->stdout_descriptor.get() >= 0 &&
                ::isatty(identifying->stdin_descriptor.get()) &&
                ::isatty(identifying->stdout_descriptor.get())
            ? StockClientLifecycle::identify_terminal(
                  server, client_id_, std::move(identifying->stdin_descriptor),
                  std::move(identifying->stdout_descriptor), compatibility)
            : StockClientLifecycle::identify_command(server, client_id_, compatibility);
    if (!client_identification) {
        throw std::runtime_error(client_identification.error().message);
    }
    phase_.emplace<AwaitingOneShotCommandPhase>(
        StockClientAdmission{std::move(selected_profile), std::move(compatibility)});
}

void AcceptedClientProtocol::invoke_client_command(Server &server,
                                                   const commands::CommandCatalog &command_catalog,
                                                   std::span<const std::uint8_t> payload,
                                                   control::CommandSequence &control_sequence) {
    if (std::holds_alternative<IdentifyingPhase>(phase_)) {
        queue_command_exit(1);
        return;
    }
    if (auto *awaiting_control = std::get_if<AwaitingControlStartCommandPhase>(&phase_)) {
        auto stock_admission = std::move(awaiting_control->admission);
        auto control_channel = std::move(awaiting_control->channel);
        auto session_event_cursor = std::move(awaiting_control->session_event_cursor);
        auto decoded_arguments = decode_command_arguments(stock_admission.profile, payload);
        auto command_outcome =
            decoded_arguments
                ? execute_registered_command(server, client_id_, command_catalog,
                                             stock_admission.compatibility, *decoded_arguments)
                : Result<commands::CommandOutcome>(std::unexpected(decoded_arguments.error()));
        const bool succeeded = command_outcome.has_value();
        const std::string command_output =
            succeeded ? std::move(command_outcome->output) : command_outcome.error().message;
        const auto control_outcome =
            succeeded ? control::CommandOutcome::succeeded : control::CommandOutcome::failed;
        if (auto command_result_queued = control_channel->queue_command_result(
                stock_admission.profile.control_mode,
                control_sequence.next(control::CommandOrigin::initial), control_outcome,
                command_output);
            !command_result_queued) {
            throw std::runtime_error(command_result_queued.error().message);
        }
        phase_.emplace<ControlModePhase>(
            std::move(stock_admission), std::move(control_channel),
            control::SessionNotificationFeed{std::move(session_event_cursor)});
        return;
    }
    auto *awaiting_command = std::get_if<AwaitingOneShotCommandPhase>(&phase_);
    if (!awaiting_command) {
        throw std::invalid_argument("stock command is invalid in the current client phase");
    }
    auto stock_admission = std::move(awaiting_command->admission);
    auto decoded_arguments = decode_command_arguments(stock_admission.profile, payload);
    auto command_outcome =
        decoded_arguments
            ? execute_registered_command(server, client_id_, command_catalog,
                                         stock_admission.compatibility, *decoded_arguments)
            : Result<commands::CommandOutcome>(std::unexpected(decoded_arguments.error()));
    const int command_status = command_outcome ? 0 : 1;
    if (command_outcome &&
        command_outcome->continuation == commands::ClientContinuation::attached) {
        if (!command_outcome->output.empty()) {
            throw std::logic_error("attached command returned command-stream output");
        }
        auto interactive_compatibility = TmuxCompatibilityPolicy{}.assess(
            stock_admission.profile, ProtocolDirection::stock_client_to_cxx_server,
            ConnectionMode::interactive);
        interactive_compatibility.commands = command_catalog.spellings();
        retain_protocol_quirks(stock_admission.compatibility, interactive_compatibility);
        if (auto compatibility_update = StockClientLifecycle::update_stock_compatibility(
                server, client_id_, interactive_compatibility);
            !compatibility_update) {
            throw std::runtime_error(compatibility_update.error().message);
        }
        queue_server_frame(
            encode_imsg(message_number(TmuxMessage::ready), {}, stock_admission.profile));
        stock_admission.compatibility = std::move(interactive_compatibility);
        phase_.emplace<AttachedTerminalPhase>(std::move(stock_admission));
        return;
    }
    std::string command_output = command_outcome ? std::move(command_outcome->output)
                                                 : command_outcome.error().message + "\n";
    if (command_output.empty()) {
        phase_.emplace<AwaitingOneShotCommandPhase>(std::move(stock_admission));
        queue_command_exit(command_status);
        return;
    }
    HostPayloadWriter open_payload(stock_admission.profile);
    open_payload.write_u32(1);
    open_payload.write_u32(command_outcome ? 1 : 2);
    open_payload.write_u32(0);
    queue_server_frame(encode_imsg(message_number(TmuxMessage::write_open), open_payload.bytes(),
                                   stock_admission.profile));
    phase_.emplace<OneShotCommandOutputPhase>(std::move(stock_admission), std::move(command_output),
                                              command_status);
}

void AcceptedClientProtocol::invoke_control_command(Server &server,
                                                    const commands::CommandCatalog &command_catalog,
                                                    control::CommandSequence &control_sequence,
                                                    std::string_view command_line) {
    auto *control_mode = std::get_if<ControlModePhase>(&phase_);
    if (!control_mode) {
        throw std::logic_error("control command requires an active control client");
    }
    auto parsed_arguments = control::parse_command_line(command_line);
    auto command_outcome =
        parsed_arguments
            ? execute_registered_command(server, client_id_, command_catalog,
                                         control_mode->admission.compatibility, *parsed_arguments)
            : Result<commands::CommandOutcome>(std::unexpected(parsed_arguments.error()));
    const bool succeeded = command_outcome.has_value();
    std::string command_output =
        succeeded ? std::move(command_outcome->output) : command_outcome.error().message;
    if (!parsed_arguments) {
        command_output.insert(0, "parse error: ");
    }
    const auto control_outcome =
        succeeded ? control::CommandOutcome::succeeded : control::CommandOutcome::failed;
    if (auto command_result_queued = control_mode->channel->queue_command_result(
            control_mode->admission.profile.control_mode,
            control_sequence.next(control::CommandOrigin::control_input), control_outcome,
            command_output);
        !command_result_queued) {
        throw std::runtime_error(command_result_queued.error().message);
    }
}

void AcceptedClientProtocol::continue_command_output(std::span<const std::uint8_t> payload) {
    auto *command_output = std::get_if<OneShotCommandOutputPhase>(&phase_);
    if (!command_output) {
        throw std::invalid_argument("stock command has no output stream awaiting acknowledgement");
    }
    const auto &wire_profile = command_output->admission.profile;
    HostPayloadReader acknowledgement(wire_profile, payload);
    if (acknowledgement.read_u32() != 1 || acknowledgement.read_u32() != 0 ||
        !acknowledgement.finished()) {
        throw std::invalid_argument("invalid stock output-stream acknowledgement");
    }
    for (std::size_t offset = 0; offset < command_output->bytes.size(); offset += 8192) {
        HostPayloadWriter data_payload(wire_profile);
        data_payload.write_u32(1);
        const auto output_fragment = std::string_view(command_output->bytes).substr(offset, 8192);
        data_payload.write_bytes(output_fragment);
        queue_server_frame(encode_imsg(message_number(TmuxMessage::write_data),
                                       data_payload.bytes(), wire_profile));
    }
    HostPayloadWriter close_payload(wire_profile);
    close_payload.write_u32(1);
    queue_server_frame(
        encode_imsg(message_number(TmuxMessage::write_close), close_payload.bytes(), wire_profile));
    const int exit_status = command_output->exit_status;
    queue_command_exit(exit_status);
}

void AcceptedClientProtocol::resize_attached_client(Server &server,
                                                    std::span<const std::uint8_t> payload) {
    if (!std::holds_alternative<AttachedTerminalPhase>(phase_) || !payload.empty()) {
        throw std::invalid_argument("stock resize requires an attached ordinary client");
    }
    auto client_resize = ResizeClient::execute(server, {client_id_});
    if (!client_resize) {
        throw std::runtime_error(client_resize.error().message);
    }
}

void AcceptedClientProtocol::complete_client_exit(Server &server,
                                                  std::span<const std::uint8_t> payload) {
    auto *attached = std::get_if<AttachedTerminalPhase>(&phase_);
    auto *detaching = std::get_if<DetachingTerminalPhase>(&phase_);
    if ((!attached && !detaching) || !payload.empty()) {
        throw std::invalid_argument("stock EXITING requires an attached ordinary client");
    }
    if (attached) {
        auto client_detachment = server.detach_client(client_id_);
        if (!client_detachment) {
            throw std::runtime_error(client_detachment.error().message);
        }
    }
    const auto &wire_profile =
        attached ? attached->admission.profile : detaching->admission.profile;
    queue_server_frame(encode_imsg(message_number(TmuxMessage::exited), {}, wire_profile));
    phase_.emplace<ClosingTransportPhase>();
}

void AcceptedClientProtocol::consume_client_message(Server &server,
                                                    const commands::CommandCatalog &command_catalog,
                                                    const StockFrameHeader &header,
                                                    std::span<const std::uint8_t> payload,
                                                    control::CommandSequence &control_sequence) {
    if ((header.version & legacy_profile.version_mask) != protocol_version) {
        queue_server_frame(encode_imsg(message_number(TmuxMessage::version)));
        phase_.emplace<ClosingTransportPhase>();
        return;
    }
    const auto &wire_profile = validation_profile();
    if (auto validation = validate_client_message(wire_profile, header, payload); !validation) {
        throw std::invalid_argument(validation.error().message);
    }
    const auto message_type = static_cast<TmuxMessage>(header.type);
    if (!std::holds_alternative<IdentifyingPhase>(phase_) &&
        is_identification_message(wire_profile, message_type)) {
        throw std::invalid_argument("stock identification is already complete");
    }
    if (header.has_descriptor) {
        adopt_identification_descriptor(message_type);
    }
    switch (message_type) {
    case TmuxMessage::identify_flags:
    case TmuxMessage::identify_longflags:
        record_client_flags(message_type, payload);
        return;
    case TmuxMessage::identify_done:
        complete_identification(server, command_catalog, payload);
        return;
    case TmuxMessage::command:
        invoke_client_command(server, command_catalog, payload, control_sequence);
        return;
    case TmuxMessage::write_ready:
        continue_command_output(payload);
        return;
    case TmuxMessage::resize:
        resize_attached_client(server, payload);
        return;
    case TmuxMessage::exiting:
        complete_client_exit(server, payload);
        return;
    default:
        if (!is_identification_message(wire_profile, message_type)) {
            throw std::invalid_argument("stock client message has no supported behavior");
        }
    }
}
} // namespace tmux_cxx::protocol::tmux
