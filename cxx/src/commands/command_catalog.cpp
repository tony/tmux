#include "commands/command_catalog.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::commands {
CommandCatalog::CommandCatalog(std::vector<Entry> commands) : commands_(std::move(commands)) {}

std::vector<std::string> CommandCatalog::spellings() const {
    std::vector<std::string> spellings;
    spellings.reserve(commands_.size());
    for (const auto &command : commands_) {
        spellings.emplace_back(command.name);
    }
    return spellings;
}

Result<CommandOutcome> CommandCatalog::invoke(CommandInvocation &invocation,
                                              std::span<const std::string> arguments) const {
    if (arguments.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "explicit registered command required"});
    }
    if (std::ranges::find(arguments, ";") != arguments.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "command groups are not yet supported"});
    }
    auto command = std::ranges::lower_bound(commands_, arguments.front(), {}, &Entry::name);
    if (command == commands_.end() || command->name != arguments.front()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "unsupported stock command"});
    }
    auto command_outcome = command->adapter(invocation, arguments.subspan(1));
    if (!command_outcome) {
        command_outcome.error().operation = command->name;
    }
    return command_outcome;
}
} // namespace tmux_cxx::commands
