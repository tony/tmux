#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/paste_buffer/operations/paste_buffer_into_pane.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate the supported stock paste-buffer subset into PasteBufferIntoPane. */
struct PasteBufferCommand {
    using Request = PasteBufferIntoPane::Request; ///< @copybrief PasteBufferIntoPane::Request
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"pasteb"};
    /** @brief Borrowed spellings and the required paste-buffer operation identity. */
    static constexpr commands::CommandDescription description{"paste-buffer", aliases,
                                                              PasteBufferIntoPane::description.id};
    /** @brief Require explicit buffer and percent-prefixed pane targets in either order. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Queue stored bytes through the shared paste-buffer operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
