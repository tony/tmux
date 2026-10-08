#include "protocol/tmux/control/command_result.hpp"

#include <ctime>

namespace tmux_cxx::protocol::tmux::control {
CommandGuard CommandSequence::next(CommandOrigin origin) {
    return {static_cast<std::int64_t>(std::time(nullptr)), ++last_number_, origin};
}

std::string format_command_result(const TmuxControlModeProfile &grammar, CommandGuard guard,
                                  CommandOutcome outcome, std::string_view body) {
    const auto guard_fields = std::to_string(guard.epoch_seconds) + " " +
                              std::to_string(guard.number) + " " +
                              (guard.origin == CommandOrigin::control_input ? "1" : "0") + "\n";
    std::string block;
    block.append(grammar.begin_guard).append(" ").append(guard_fields);
    block.append(body);
    if (!body.empty() && body.back() != '\n') {
        block.push_back('\n');
    }
    block.append(outcome == CommandOutcome::succeeded ? grammar.end_guard : grammar.error_guard)
        .append(" ")
        .append(guard_fields);
    return block;
}
} // namespace tmux_cxx::protocol::tmux::control
