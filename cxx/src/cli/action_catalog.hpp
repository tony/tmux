#pragma once

#include <tmux_cxx/tmux_error.hpp>

#include <iosfwd>
#include <span>
#include <string_view>
#include <vector>

namespace tmux_cxx {
class ServerConnection;
namespace cli {
class ActionRegistry;

/** @brief Describe one registered process command-line action. */
struct ActionDescription {
    std::string_view name;       ///< Stable command-line spelling.
    std::string_view operands{}; ///< Operand synopsis following the action spelling.
};

/** @brief Supply the native server controller and command-line output destination. */
struct ActionInvocation {
    ServerConnection &connection; ///< Controller shared by the selected action.
    std::ostream &output;         ///< Process standard output or a test-owned substitute.
};

/** @brief Provide immutable process-action lookup after startup registration succeeds. */
class ActionCatalog {
  public:
    /** @brief Parse and invoke one action selected by the first argument. */
    Result<void> invoke(ActionInvocation &invocation,
                        std::span<const std::string_view> arguments) const;
    /** @brief Write each action and its operand synopsis with a caller-owned prefix. */
    Result<void> write_usage(std::ostream &output, std::string_view prefix) const;

  private:
    friend class ActionRegistry;
    /** @brief Erase typed request handling only at immutable CLI lookup. */
    using ActionAdapter = Result<void> (*)(ActionInvocation &, std::span<const std::string_view>);
    /** @brief Pair one action spelling with its typed invocation adapter. */
    struct Entry {
        std::string_view name;     ///< Immutable command-line spelling.
        std::string_view operands; ///< Immutable operand synopsis for process help.
        ActionAdapter adapter;     ///< Typed decoder and execution entry point.
    };
    /** @brief Adopt validated actions already sorted by process spelling. */
    explicit ActionCatalog(std::vector<Entry> actions);
    std::vector<Entry> actions_; ///< Frozen process actions sorted by spelling.
};

/** @brief Build the immutable catalog of entity-owned command-line adapters. */
Result<ActionCatalog> build_action_catalog();
} // namespace cli
} // namespace tmux_cxx
