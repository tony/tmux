#pragma once

#include <tmux_cxx/server/server.hpp>

#include <optional>

namespace tmux_cxx {
/** @brief Retain a textual session target and optional window index until command execution. */
struct SessionWindowTarget {
    std::string session; ///< Exact session name or dollar-prefixed identity.
    std::optional<std::uint32_t>
        index; ///< Explicit index, absent when creation chooses a free one.
};
/** @brief Parse session[:index], rejecting malformed or out-of-range indices without resolving
 * entities. */
Result<SessionWindowTarget> parse_session_window_target(std::string_view target);
/** @brief Resolve an exact indexed membership against the current server graph. */
Result<WindowLinkSnapshot> resolve_window_link_target(const Server &server,
                                                      const SessionWindowTarget &target);
} // namespace tmux_cxx
