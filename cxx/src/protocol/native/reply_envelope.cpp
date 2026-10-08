#include "protocol/native/reply_envelope.hpp"
#include "text/utf8.hpp"

#include <stdexcept>
#include <utility>

namespace tmux_cxx::protocol::native {
namespace {
/** @brief Validate the native protocol's lowercase hexadecimal server-instance representation. */
bool valid_server_instance(std::string_view value) {
    if (value.size() != 32) {
        return false;
    }
    for (const auto character : value) {
        if (!(character >= '0' && character <= '9') && !(character >= 'a' && character <= 'f')) {
            return false;
        }
    }
    return true;
}
} // namespace

Bytes encode_reply_envelope(std::string_view server_instance,
                            const Result<Bytes> &operation_result) {
    if (!valid_server_instance(server_instance)) {
        throw std::invalid_argument("invalid native server identity");
    }
    NativePayloadWriter writer;
    writer.string(server_instance);
    if (operation_result) {
        writer.u16(static_cast<std::uint16_t>(TmuxErrorCode::none));
        writer.string("");
        writer.bytes.insert(writer.bytes.end(), operation_result->begin(), operation_result->end());
    } else {
        const auto &failure = operation_result.error();
        if (failure.code == TmuxErrorCode::none ||
            tmux_error_code_name(failure.code) == "unknown" || !text::valid_utf8(failure.message)) {
            throw std::invalid_argument("invalid native failure metadata");
        }
        writer.u16(static_cast<std::uint16_t>(failure.code));
        writer.string(failure.message);
    }
    if (writer.bytes.size() > max_payload) {
        throw std::length_error("native reply envelope exceeds frame capacity");
    }
    return std::move(writer.bytes);
}

Result<ReplyEnvelope> decode_reply_envelope(std::span<const std::uint8_t> envelope_bytes) {
    try {
        if (envelope_bytes.size() > max_payload) {
            throw std::length_error("native reply envelope exceeds frame capacity");
        }
        NativePayloadReader reader(envelope_bytes);
        auto server_instance = reader.string();
        if (!valid_server_instance(server_instance)) {
            throw std::invalid_argument("invalid native server identity");
        }
        const auto error_code = static_cast<TmuxErrorCode>(reader.u16());
        auto error_message = reader.string();
        if (tmux_error_code_name(error_code) == "unknown" || !text::valid_utf8(error_message)) {
            throw std::invalid_argument("invalid native failure metadata");
        }
        if (error_code != TmuxErrorCode::none) {
            reader.require_end();
            TmuxError failure{error_code, std::move(error_message), {}, server_instance};
            return ReplyEnvelope{std::move(server_instance), std::unexpected(std::move(failure))};
        }
        if (!error_message.empty()) {
            throw std::invalid_argument("successful native reply contains failure text");
        }
        const auto payload = reader.remaining();
        return ReplyEnvelope{std::move(server_instance), Bytes(payload.begin(), payload.end())};
    } catch (const std::exception &failure) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, failure.what()});
    }
}
} // namespace tmux_cxx::protocol::native
