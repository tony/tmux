#pragma once

#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>

#include <cstdint>
#include <string>
#include <string_view>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Distinguish the packed startup command from descriptor-read commands. */
enum class CommandOrigin {
    initial,       ///< Command carried by the stock socket after identification.
    control_input, ///< Command read from this client's control input descriptor.
};

/** @brief Select the closing guard for one control-command result block. */
enum class CommandOutcome {
    succeeded, ///< Close the result block with the success guard.
    failed,    ///< Close the result block with the error guard.
};

/** @brief Identify one command block in the shared stock control stream. */
struct CommandGuard {
    std::int64_t epoch_seconds; ///< Wall-clock execution time required by stock grammar.
    std::uint32_t number;       ///< Server-wide command number with defined unsigned wrap.
    CommandOrigin origin;       ///< Source encoded in the stock guard flags field.
};

/** @brief Allocate server-wide stock control command guards. */
class CommandSequence {
  public:
    /** @brief Allocate the next guard with current wall-clock time and shared numbering. */
    CommandGuard next(CommandOrigin origin);

  private:
    std::uint32_t last_number_ = 0; ///< Most recently allocated command number.
};

/** @brief Format one guarded command result under the selected stock grammar. */
std::string format_command_result(const TmuxControlModeProfile &grammar, CommandGuard guard,
                                  CommandOutcome outcome, std::string_view body);
} // namespace tmux_cxx::protocol::tmux::control
