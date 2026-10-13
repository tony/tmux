#pragma once

#include "commands/command_registry.hpp"
#include "window_link/window_target.hpp"
#include <tmux_cxx/window/operations/select_layout.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Resolve a stock window target before selecting its named pane arrangement. */
struct SelectLayoutCommand {
    /** @brief Retain an explicit membership target and a validated exact preset name. */
    struct Request {
        SessionWindowTarget target; ///< Exact session and window index.
        WindowLayoutPreset preset;  ///< Shared-window arrangement selected by its stock name.
    };
    static constexpr std::array<std::string_view, 1> aliases{"selectl"};  ///< Stock tmux alias.
    static constexpr std::array operations{SelectLayout::description.id}; ///< Required operation.
    /** @brief Borrow stock spellings and the typed layout dependency for startup validation. */
    static constexpr commands::CommandDescription description{"select-layout", aliases, operations};
    /** @brief Require an explicit membership target and one exact supported layout name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Resolve the current membership and invoke SelectLayout on its shared window. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
