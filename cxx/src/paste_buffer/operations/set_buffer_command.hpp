#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate the supported stock set-buffer subset into SetPasteBuffer. */
struct SetBufferCommand {
    using Request = SetPasteBuffer::Request; ///< @copybrief SetPasteBuffer::Request
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"setb"};
    /** @brief Borrowed spellings and the required paste-buffer operation identity. */
    static constexpr commands::CommandDescription description{"set-buffer", aliases,
                                                              SetPasteBuffer::description.id};
    /** @brief Require one explicit buffer name and one data argument. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Store bytes through the shared paste-buffer operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
