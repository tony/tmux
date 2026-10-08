#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/session/operations/list_sessions.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate stock list-sessions into shared observation and command-owned formatting. */
struct ListSessionsCommand {
    /** @brief Stock format expression applied to materialized session values. */
    struct Request {
        std::string expression; ///< Supported stock format expression.
    };
    /** @brief Declared alternate stock spellings for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"ls"};
    /** @brief Borrowed spellings and the required native operation identity. */
    static constexpr commands::CommandDescription description{"list-sessions", aliases,
                                                              ListSessions::description.id};
    /** @brief Accept an optional format expression and reject undeclared options. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Format materialized snapshots without reparsing substituted session names. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
