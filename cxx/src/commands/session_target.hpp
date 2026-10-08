#pragma once

#include <tmux_cxx/server/server.hpp>

namespace tmux_cxx::commands {
/** @brief Resolve an exact session name or dollar-prefixed identity at command execution time. */
Result<SessionId> resolve_session_target(const Server &server, std::string_view target);
} // namespace tmux_cxx::commands
