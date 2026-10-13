#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/paste_buffer/operations/paste_buffer_into_pane.hpp>
#include <tmux_cxx/paste_buffer/operations/paste_top_buffer.hpp>

#include <array>
#include <variant>

namespace tmux_cxx {
/** @brief Translate named or default stock paste into its typed pane-input operation. */
struct PasteBufferCommand {
    using Request = std::variant<PasteBufferIntoPane::Request, PasteTopBuffer::Request>;
    ///< Explicit named lookup or the newest automatic buffer.
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"pasteb"};
    static constexpr std::array operations{PasteBufferIntoPane::description.id,
                                           PasteTopBuffer::description.id};
    ///< Native operations reachable through named and default paste.
    /** @brief Borrowed spellings and the required paste-buffer operation identity. */
    static constexpr commands::CommandDescription description{"paste-buffer", aliases, operations};
    /** @brief Require a percent-prefixed pane target with an optional explicit buffer name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Queue stored bytes through the shared paste-buffer operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
