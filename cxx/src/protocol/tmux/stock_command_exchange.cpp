#include "protocol/tmux/stock_command_exchange.hpp"

#include "protocol/tmux/payload_codec.hpp"

#include <algorithm>
#include <cerrno>
#include <stdexcept>

namespace tmux_cxx::protocol::tmux {
namespace {
constexpr std::size_t command_output_limit = 1024 * 1024; ///< Bound captured output per command.

/** @brief Convert one descriptor source into the marker required by stock framing. */
DescriptorTransfer descriptor_transfer(StockClientDescriptor descriptor) {
    return descriptor == StockClientDescriptor::none ? DescriptorTransfer::none
                                                     : DescriptorTransfer::follows;
}
} // namespace

StockClientFrame StockCommandExchange::encode_client_frame(TmuxMessage message,
                                                           const Bytes &payload,
                                                           StockClientDescriptor descriptor) const {
    auto frame_bytes = encode_imsg(message_number(message), payload, selected_profile_,
                                   descriptor_transfer(descriptor));
    const auto decoded_header = decode_imsg_header(frame_bytes);
    if (!decoded_header) {
        throw std::logic_error(decoded_header.error().message);
    }
    const auto validation = validate_client_message(
        selected_profile_, *decoded_header,
        std::span(frame_bytes).subspan(imsg_header_bytes, frame_bytes.size() - imsg_header_bytes));
    if (!validation) {
        throw std::logic_error(validation.error().message);
    }
    return {std::move(frame_bytes), descriptor};
}

void StockCommandExchange::append_identification_step(
    std::vector<StockClientFrame> &frames, const StockClientIdentification &identification,
    const TmuxIdentificationStep &step) const {
    HostPayloadWriter payload(selected_profile_);
    switch (step.value) {
    case TmuxIdentificationValue::client_flags:
        if (step.message == TmuxMessage::identify_flags) {
            payload.write_u32(static_cast<std::uint32_t>(identification.client_flags));
        } else {
            payload.write_u64(identification.client_flags);
        }
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::terminal_type:
        payload.write_c_string(identification.terminal_type);
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::terminal_features:
        payload.write_u32(identification.terminal_features);
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::tty_name:
        payload.write_c_string(identification.tty_name);
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::working_directory:
        payload.write_c_string(identification.working_directory);
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::terminal_capabilities:
        for (const auto &capability : identification.terminal_capabilities) {
            HostPayloadWriter capability_payload(selected_profile_);
            capability_payload.write_c_string(capability);
            frames.push_back(encode_client_frame(step.message, capability_payload.bytes()));
        }
        break;
    case TmuxIdentificationValue::standard_input:
        frames.push_back(
            encode_client_frame(step.message, {}, StockClientDescriptor::standard_input));
        break;
    case TmuxIdentificationValue::standard_output:
        frames.push_back(
            encode_client_frame(step.message, {}, StockClientDescriptor::standard_output));
        break;
    case TmuxIdentificationValue::process_id:
        payload.write_u32(identification.process_id);
        frames.push_back(encode_client_frame(step.message, payload.bytes()));
        break;
    case TmuxIdentificationValue::environment:
        for (const auto &environment_entry : identification.environment) {
            HostPayloadWriter environment_payload(selected_profile_);
            environment_payload.write_c_string(environment_entry);
            frames.push_back(encode_client_frame(step.message, environment_payload.bytes()));
        }
        break;
    case TmuxIdentificationValue::completion:
        frames.push_back(encode_client_frame(step.message));
        break;
    }
}

Result<std::vector<StockClientFrame>>
StockCommandExchange::begin_command(const StockClientIdentification &identification,
                                    std::span<const std::string> arguments) {
    try {
        if (command_started_) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "stock command was already started"});
        }
        std::vector<StockClientFrame> client_frames;
        std::size_t identification_bytes = 0;
        for (const auto &step : selected_profile_.identification.sequence) {
            const auto first_appended_frame = client_frames.size();
            append_identification_step(client_frames, identification, step);
            for (std::size_t index = first_appended_frame; index < client_frames.size(); ++index) {
                identification_bytes += client_frames[index].bytes.size();
            }
        }
        if (identification_bytes > selected_profile_.identification.max_bytes) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "stock identification exceeds its profile bound"});
        }
        auto command_payload = encode_command_arguments(selected_profile_, arguments);
        if (!command_payload) {
            return std::unexpected(command_payload.error());
        }
        client_frames.push_back(encode_client_frame(TmuxMessage::command, *command_payload));
        command_started_ = true;
        return client_frames;
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}

void StockCommandExchange::acknowledge_write_stream(std::uint32_t stream_id,
                                                    std::uint32_t error_number,
                                                    ServerTrafficOutcome &outcome) const {
    HostPayloadWriter payload(selected_profile_);
    payload.write_u32(stream_id);
    payload.write_u32(error_number);
    outcome.client_replies.push_back(
        encode_client_frame(TmuxMessage::write_ready, payload.bytes()));
}

Result<void> StockCommandExchange::open_output_stream(std::span<const std::uint8_t> payload,
                                                      ServerTrafficOutcome &outcome) {
    const auto stream_fields_bytes = 3 * selected_profile_.payload_abi.int_bytes;
    HostPayloadReader stream_fields(selected_profile_, payload.first(stream_fields_bytes));
    const auto stream_id = stream_fields.read_u32();
    const auto file_descriptor = stream_fields.read_u32();
    stream_fields.read_u32();
    const auto existing_stream = std::ranges::find(open_streams_, stream_id, &OpenStream::id);
    const auto path_payload = payload.subspan(stream_fields_bytes);
    const bool captures_standard_stream =
        path_payload.empty() ||
        (path_payload.size() == 2 && path_payload[0] == '-' && path_payload[1] == 0);
    if (existing_stream != open_streams_.end()) {
        acknowledge_write_stream(stream_id, EBUSY, outcome);
    } else if (!captures_standard_stream || stream_id != file_descriptor ||
               (stream_id != 1 && stream_id != 2)) {
        acknowledge_write_stream(stream_id, ENOTSUP, outcome);
    } else {
        open_streams_.push_back({stream_id, file_descriptor == 1 ? StreamTarget::standard_output
                                                                 : StreamTarget::standard_error});
        acknowledge_write_stream(stream_id, 0, outcome);
    }
    return {};
}

Result<void>
StockCommandExchange::capture_output_stream_data(std::span<const std::uint8_t> payload) {
    HostPayloadReader stream_reader(selected_profile_,
                                    payload.first(selected_profile_.payload_abi.int_bytes));
    const auto stream_id = stream_reader.read_u32();
    const auto open_stream = std::ranges::find(open_streams_, stream_id, &OpenStream::id);
    if (open_stream == open_streams_.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock server wrote an unknown stream"});
    }
    auto &captured_output = open_stream->target == StreamTarget::standard_output
                                ? pending_command_result_.standard_output
                                : pending_command_result_.standard_error;
    const auto stream_data = payload.subspan(selected_profile_.payload_abi.int_bytes);
    const auto captured_bytes = pending_command_result_.standard_output.size() +
                                pending_command_result_.standard_error.size();
    if (captured_bytes > command_output_limit ||
        stream_data.size() > command_output_limit - captured_bytes) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock command output exceeds capacity"});
    }
    captured_output.insert(captured_output.end(), stream_data.begin(), stream_data.end());
    return {};
}

Result<void> StockCommandExchange::close_output_stream(std::span<const std::uint8_t> payload) {
    HostPayloadReader stream_reader(selected_profile_, payload);
    const auto stream_id = stream_reader.read_u32();
    stream_reader.require_end();
    const auto open_stream = std::ranges::find(open_streams_, stream_id, &OpenStream::id);
    if (open_stream == open_streams_.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock server closed an unknown stream"});
    }
    open_streams_.erase(open_stream);
    return {};
}

void StockCommandExchange::complete_command(std::span<const std::uint8_t> payload,
                                            ServerTrafficOutcome &outcome) {
    if (!payload.empty()) {
        HostPayloadReader status(selected_profile_,
                                 payload.first(selected_profile_.payload_abi.int_bytes));
        pending_command_result_.exit_status = static_cast<std::int32_t>(status.read_u32());
        const auto exit_message_bytes = payload.subspan(selected_profile_.payload_abi.int_bytes);
        if (!exit_message_bytes.empty()) {
            pending_command_result_.exit_message.assign(
                reinterpret_cast<const char *>(exit_message_bytes.data()),
                exit_message_bytes.size());
            pending_command_result_.exit_message.back() = '\0';
            pending_command_result_.exit_message.resize(
                pending_command_result_.exit_message.find('\0'));
        }
    }
    outcome.completed_command = std::move(pending_command_result_);
    command_finished_ = true;
}

Result<void> StockCommandExchange::handle_server_message(const StockFrameHeader &header,
                                                         std::span<const std::uint8_t> payload,
                                                         ServerTrafficOutcome &outcome) {
    const auto message_type = static_cast<TmuxMessage>(header.type);
    switch (message_type) {
    case TmuxMessage::write_open:
        return open_output_stream(payload, outcome);
    case TmuxMessage::write_data:
        return capture_output_stream_data(payload);
    case TmuxMessage::write_close:
        return close_output_stream(payload);
    case TmuxMessage::exit:
    case TmuxMessage::shutdown:
        complete_command(payload, outcome);
        return {};
    case TmuxMessage::flags:
        return {};
    case TmuxMessage::version:
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "stock server rejected the selected profile"});
    default:
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "stock server message is outside command mode"});
    }
}

Result<StockCommandExchange::ServerTrafficOutcome>
StockCommandExchange::receive_server_traffic(std::span<const std::uint8_t> server_bytes) {
    try {
        if (!command_started_ || command_finished_) {
            return std::unexpected(TmuxError{TmuxErrorCode::protocol,
                                             "stock server traffic arrived outside a command"});
        }
        if (inbound_bytes_.size() + server_bytes.size() >
            command_output_limit + selected_profile_.max_frame_bytes) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "stock server input exceeds capacity"});
        }
        inbound_bytes_.insert(inbound_bytes_.end(), server_bytes.begin(), server_bytes.end());
        ServerTrafficOutcome traffic_result;
        while (inbound_bytes_.size() >= imsg_header_bytes && !command_finished_) {
            const auto decoded_header = decode_imsg_header(inbound_bytes_);
            if (!decoded_header) {
                return std::unexpected(decoded_header.error());
            }
            if ((decoded_header->version & selected_profile_.version_mask) !=
                selected_profile_.protocol_version) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::unsupported, "stock server protocol version differs"});
            }
            if (inbound_bytes_.size() < decoded_header->length) {
                break;
            }
            const auto payload =
                std::span(inbound_bytes_)
                    .subspan(imsg_header_bytes, decoded_header->length - imsg_header_bytes);
            if (auto validation =
                    validate_server_message(selected_profile_, *decoded_header, payload);
                !validation) {
                return std::unexpected(validation.error());
            }
            if (auto handling = handle_server_message(*decoded_header, payload, traffic_result);
                !handling) {
                return std::unexpected(handling.error());
            }
            inbound_bytes_.erase(inbound_bytes_.begin(),
                                 inbound_bytes_.begin() + decoded_header->length);
        }
        if (command_finished_ && !inbound_bytes_.empty()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "stock server sent traffic after command exit"});
        }
        return traffic_result;
    } catch (const std::exception &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}
} // namespace tmux_cxx::protocol::tmux
