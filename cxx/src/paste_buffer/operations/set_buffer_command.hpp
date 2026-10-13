#pragma once

#include "commands/command_registry.hpp"
#include <tmux_cxx/paste_buffer/operations/add_paste_buffer.hpp>
#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

#include <array>
#include <variant>

namespace tmux_cxx {
/** @brief Translate named or automatic stock storage into its typed paste-buffer operation. */
struct SetBufferCommand {
    using Request = std::variant<SetPasteBuffer::Request, AddPasteBuffer::Request>;
    ///< Explicit named storage or automatic name allocation.
    /** @brief Declared alternate stock spelling for this command. */
    static constexpr std::array<std::string_view, 1> aliases{"setb"};
    static constexpr std::array operations{SetPasteBuffer::description.id,
                                           AddPasteBuffer::description.id};
    ///< Native operations reachable through named and default storage.
    /** @brief Borrowed spellings and the required paste-buffer operation identity. */
    static constexpr commands::CommandDescription description{"set-buffer", aliases, operations};
    /** @brief Require one data argument with an optional explicit buffer name. */
    static Result<Request> read(std::span<const std::string> arguments);
    /** @brief Store bytes through the shared paste-buffer operation. */
    static Result<commands::CommandOutcome> execute(commands::CommandInvocation &invocation,
                                                    Request request);
};
} // namespace tmux_cxx
