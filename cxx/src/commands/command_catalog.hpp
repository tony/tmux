#pragma once

#include "commands/command.hpp"

#include <tmux_cxx/tmux_error.hpp>

#include <span>
#include <string_view>
#include <vector>

namespace tmux_cxx::commands {
class CommandRegistry;

/** @brief Provide immutable stock-command lookup after startup registration succeeds. */
class CommandCatalog {
  public:
    /** @brief Invoke a supported spelling without command-specific transport branching. */
    Result<CommandOutcome> invoke(CommandInvocation &invocation,
                                  std::span<const std::string> arguments) const;
    /** @brief Copy supported command spellings and aliases for compatibility reports. */
    std::vector<std::string> spellings() const;

  private:
    friend class CommandRegistry;
    /** @brief Erase typed argument parsing and execution only at immutable lookup. */
    using CommandAdapter = Result<CommandOutcome> (*)(CommandInvocation &,
                                                      std::span<const std::string>);
    /** @brief Pair one supported command spelling with its validated invocation adapter. */
    struct Entry {
        std::string_view name;  ///< Canonical spelling or explicitly registered alias.
        CommandAdapter adapter; ///< Typed parsing and execution entry point.
    };
    /** @brief Adopt validated command entries already sorted by spelling. */
    explicit CommandCatalog(std::vector<Entry> commands);
    std::vector<Entry> commands_; ///< Frozen command entries sorted by spelling.
};
} // namespace tmux_cxx::commands
