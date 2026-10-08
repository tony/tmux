#pragma once

#include <tmux_cxx/protocol/bytes.hpp>
#include <tmux_cxx/protocol/native/capability.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <chrono>
#include <compare>
#include <functional>
#include <optional>
#include <variant>

namespace tmux_cxx {
class Server;
}
namespace tmux_cxx::protocol::native {
using protocol::Bytes;
using Clock =
    std::chrono::steady_clock; ///< Monotonic request deadlines independent of wall-clock changes.
class NativePayloadReader;

/** @brief Identify a registered native API operation within one protocol version. */
struct OperationId {
    std::uint16_t value{}; ///< Native API identity within the protocol version.
    /** @brief Compare native operation identities without conflating them with entity identities.
     */
    auto operator<=>(const OperationId &) const = default;
};
/** @brief Distinguish observation, server mutation, and program-input effects. */
enum class OperationEffect { query, mutation, program_input };
/** @brief Describe immutable native operation metadata borrowed during startup and runtime lookup.
 */
struct OperationDescription {
    OperationId id;                          ///< Wire identity validated before lookup freezes.
    std::string_view name;                   ///< Stable entity/action name used in error context.
    OperationEffect effect;                  ///< Effect classification for this operation.
    std::uint64_t required_capabilities = 0; ///< Native server behavior needed before execution.
};

/** @brief A pending request owns its deadline and continuation; scheduling does not reverse prior
 * effects. */
struct PendingReply {
    Clock::time_point deadline; ///< Absolute steady-clock deadline owned by the operation.
    std::move_only_function<Result<std::optional<Bytes>>(Server &)>
        resume; ///< Server-thread continuation owned by this pending reply.
};
using OperationReply =
    std::variant<Bytes, PendingReply>; ///< Immediate payload or operation-owned continuation.
} // namespace tmux_cxx::protocol::native
