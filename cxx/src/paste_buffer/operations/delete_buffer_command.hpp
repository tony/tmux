#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/paste_buffer/operations/delete_paste_buffer.hpp>

#include <array>

namespace tmux_cxx {
/** @brief Translate the supported stock delete-buffer subset into DeletePasteBuffer. */
struct DeleteBufferCommand {
    using Request = DeletePasteBuffer::Request; ///< @copybrief DeletePasteBuffer::Request
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"deleteb"};
    static constexpr std::array operations{
        DeletePasteBuffer::description.id}; ///< Required native operation.
    /** @brief Borrowed spellings and the required paste-buffer operation identity. */
    static constexpr commands::CommandDescription description{"delete-buffer", aliases, operations};
    /** @brief Require one explicit buffer name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Delete through the shared paste-buffer operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
