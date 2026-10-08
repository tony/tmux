#pragma once

#include "commands/command_catalog.hpp"
#include "protocol/native/operation_catalog.hpp"

#include <concepts>

namespace tmux_cxx::commands {
/** @brief Describe a stock command's spelling, aliases, and required native operation. */
struct CommandDescription {
    std::string_view name; ///< Canonical spelling or explicitly registered stock alias.
    std::span<const std::string_view> aliases; ///< Borrowed immutable alternate spellings.
    protocol::native::OperationId operation;   ///< Native identity required by this command.
};

/** @brief Require typed argument parsing before a stock command reaches server behavior. */
template <class Command>
concept RegisteredCommand =
    requires(CommandInvocation &invocation, std::span<const std::string> arguments,
             typename Command::Request request) {
        { Command::description } -> std::convertible_to<CommandDescription>;
        { Command::read(arguments) } -> std::same_as<Result<typename Command::Request>>;
        {
            Command::execute(invocation, std::move(request))
        } -> std::same_as<Result<CommandOutcome>>;
    };

/** @brief Validate stock spellings and native references before freezing command lookup. */
class CommandRegistry {
  public:
    /** @brief Register one typed command whose immutable description outlives the resulting
     * catalog. */
    template <RegisteredCommand Command> Result<void> add() {
        /** @brief Parse this command's arguments before invoking its typed execution entry point.
         */
        auto command_adapter =
            +[](CommandInvocation &invocation,
                std::span<const std::string> arguments) -> Result<CommandOutcome> {
            auto request = Command::read(arguments);
            if (!request) {
                return std::unexpected(request.error());
            }
            return Command::execute(invocation, std::move(*request));
        };
        return add_registration(Command::description, command_adapter);
    }
    /** @brief Consume registrations after every command resolves to a native operation. */
    Result<CommandCatalog> freeze(const protocol::native::OperationCatalog &operations) &&;

  private:
    /** @brief Map one declared command spelling to its native requirement and invocation. */
    struct Registration {
        std::string_view name; ///< Canonical spelling or explicitly registered stock alias.
        protocol::native::OperationId
            operation; ///< Required native identity validated before freezing.
        CommandCatalog::CommandAdapter adapter; ///< Typed parsing and execution entry point.
    };
    /** @brief Reject duplicate names and aliases atomically before extending the catalog. */
    Result<void> add_registration(CommandDescription description,
                                  CommandCatalog::CommandAdapter adapter);
    std::vector<Registration> registrations_; ///< Mutable startup registrations.
    bool registration_closed_ = false;        ///< Prevent reuse after catalog construction.
};
} // namespace tmux_cxx::commands
