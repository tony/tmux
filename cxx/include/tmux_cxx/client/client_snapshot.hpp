#pragma once

#include <tmux_cxx/identity.hpp>

#include <cstdint>
#include <optional>
#include <string>

namespace tmux_cxx {
/** @brief Copy one client's attachment and terminal size without exposing live resources. */
struct ClientSnapshot {
    /** @brief Stable identity for this server-side client lifetime. */
    ClientId id;
    /** @brief Attached retained session, absent while detached. */
    std::optional<SessionId> session;
    /** @brief Session name reported by the latest completed detachment. */
    std::string detached_from;
    /** @brief Stock identification completed for this client. */
    bool identified;
    /** @brief An ordinary terminal route is available for attachment. */
    bool has_terminal;
    /** @brief Outer terminal rows, or zero without a usable terminal. */
    std::uint32_t rows;
    /** @brief Outer terminal columns, or zero without a usable terminal. */
    std::uint32_t columns;
    /** @brief Most recent client mutation in the server sequence. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
