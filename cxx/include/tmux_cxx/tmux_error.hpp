#pragma once

#include <cstdint>
#include <expected>
#include <string>
#include <string_view>

namespace tmux_cxx {
/** @brief Stable failure classifications preserved across the native protocol and both language
 * bindings. */
enum class TmuxErrorCode : std::uint16_t {
    none,
    not_found,
    duplicate_name,
    invalid_argument,
    protocol,
    io,
    timeout,
    closed,
    wrong_server,
    output_gap,
    duplicate_index,
    unsupported,
    backpressure,
    observation_gap,
    cancelled,
};

/** @brief A recoverable tmux failure; context identifies the request, without promising rollback.
 */
struct TmuxError {
    TmuxErrorCode code;            ///< Stable machine-readable failure classification.
    std::string message;           ///< Human-readable context from the failed operation.
    std::string operation{};       ///< Requested operation identity when known.
    std::string server_instance{}; ///< Observed server identity when a reply supplied it.
};
/** @brief Return recoverable tmux failures explicitly while retaining operation context. */
template <class Value> using Result = std::expected<Value, TmuxError>;

/** @brief Return the stable machine-readable name of a tmux failure classification. */
constexpr std::string_view tmux_error_code_name(TmuxErrorCode code) noexcept {
    switch (code) {
    case TmuxErrorCode::none:
        return "none";
    case TmuxErrorCode::not_found:
        return "not_found";
    case TmuxErrorCode::duplicate_name:
        return "duplicate_name";
    case TmuxErrorCode::invalid_argument:
        return "invalid_argument";
    case TmuxErrorCode::protocol:
        return "protocol";
    case TmuxErrorCode::io:
        return "io";
    case TmuxErrorCode::timeout:
        return "timeout";
    case TmuxErrorCode::closed:
        return "closed";
    case TmuxErrorCode::wrong_server:
        return "wrong_server";
    case TmuxErrorCode::output_gap:
        return "output_gap";
    case TmuxErrorCode::duplicate_index:
        return "duplicate_index";
    case TmuxErrorCode::unsupported:
        return "unsupported";
    case TmuxErrorCode::backpressure:
        return "backpressure";
    case TmuxErrorCode::observation_gap:
        return "observation_gap";
    case TmuxErrorCode::cancelled:
        return "cancelled";
    }
    return "unknown";
}
} // namespace tmux_cxx
