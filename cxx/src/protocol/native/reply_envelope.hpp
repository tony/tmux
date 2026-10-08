#pragma once

#include "protocol/native/framing.hpp"

namespace tmux_cxx::protocol::native {
/** @brief Own validated server identity and an operation result independently of encoded reply
 * bytes. */
struct ReplyEnvelope {
    std::string server_instance; ///< Validated 32-character lowercase hexadecimal server identity.
    Result<Bytes> operation_result; ///< Copied bytes or a tmux failure with observed ownership.
};

/** @brief Encode server ownership and one result; invalid metadata throws invalid_argument and
 * excessive bytes throw length_error. */
Bytes encode_reply_envelope(std::string_view server_instance,
                            const Result<Bytes> &operation_result);
/** @brief Decode ownership and declared failure classifications before callers pin identity; reject
 * contradictory or excess error bytes. */
Result<ReplyEnvelope> decode_reply_envelope(std::span<const std::uint8_t> envelope_bytes);
} // namespace tmux_cxx::protocol::native
