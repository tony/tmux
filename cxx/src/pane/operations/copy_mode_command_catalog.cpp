#include "pane/operations/copy_mode_command_catalog.hpp"

#include <tmux_cxx/pane/operations/read_copy_mode.hpp>

#include <algorithm>
#include <type_traits>
#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Convert one uncounted action's concrete request into the closed copy-command union. */
template <class Command> CopyModeCommandRequest read_uncounted_copy_command(PaneId pane) {
    return Command::read(pane);
}

/** @brief Convert one repeated motion's concrete request while retaining its validated count. */
template <class Command>
CopyModeCommandRequest read_repeated_copy_command(PaneId pane, std::uint32_t count) {
    return Command::read(pane, count);
}

/** @brief Describe a typed stock action with the factory matching its native argument shape. */
template <class Command> constexpr CopyModeCommandDescription describe_copy_command() {
    if constexpr (requires { Command::read(PaneId{}, std::uint32_t{}); }) {
        return {Command::name, Command::Operation::description.id,
                &read_repeated_copy_command<Command>};
    } else {
        return {Command::name, Command::Operation::description.id,
                &read_uncounted_copy_command<Command>};
    }
}

constexpr std::array command_catalog{
    describe_copy_command<CopyCursorCommand<CopyCursorDirection::left>>(),
    describe_copy_command<CopyCursorCommand<CopyCursorDirection::right>>(),
    describe_copy_command<CopyCursorCommand<CopyCursorDirection::up>>(),
    describe_copy_command<CopyCursorCommand<CopyCursorDirection::down>>(),
    describe_copy_command<CopyCursorDestinationCommand<CopyCursorDestination::line_start>>(),
    describe_copy_command<CopyCursorDestinationCommand<CopyCursorDestination::line_end>>(),
    describe_copy_command<CopyCursorDestinationCommand<CopyCursorDestination::history_start>>(),
    describe_copy_command<CopyCursorDestinationCommand<CopyCursorDestination::history_end>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::line_up>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::line_down>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::half_page_up>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::half_page_down>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::page_up>>(),
    describe_copy_command<CopyScrollCommand<CopyScrollMotion::page_down>>(),
    describe_copy_command<BeginCopySelectionCommand>(),
    describe_copy_command<ClearCopySelectionCommand>(),
    describe_copy_command<CopySelectionCommand<CopySelectionDisposition::keep_selection>>(),
    describe_copy_command<CopySelectionCommand<CopySelectionDisposition::clear_selection>>(),
    describe_copy_command<CopySelectionCommand<CopySelectionDisposition::cancel_mode>>(),
    describe_copy_command<CancelCopyModeCommand>()};
///< Immutable supported actions; each entry's typed factory supplies its native dependency.

/** @brief Reject empty or repeated stock action names before they can enter the command catalog. */
consteval bool valid_copy_command_names() {
    for (std::size_t index = 0; index < command_catalog.size(); ++index) {
        if (command_catalog[index].name.empty()) {
            return false;
        }
        for (std::size_t previous = 0; previous < index; ++previous) {
            if (command_catalog[previous].name == command_catalog[index].name) {
                return false;
            }
        }
    }
    return true;
}

static_assert(valid_copy_command_names(), "copy-mode command names must be nonempty and unique");

constexpr auto required_operations =
    /** @brief Derive unique native operation dependencies from the action catalog. */
    [] {
        std::array<protocol::native::OperationId, command_catalog.size() + 1> operations{};
        std::size_t count = 0;
        /** @brief Append an operation once while preserving stable catalog declaration order. */
        const auto add = [&](protocol::native::OperationId operation) {
            const auto retained = std::span{operations}.first(count);
            if (std::ranges::find(retained, operation) == retained.end()) {
                operations[count++] = operation;
            }
        };
        for (const auto &command : command_catalog) {
            add(command.operation);
        }
        add(ReadCopyMode::description.id);
        return std::pair{operations, count};
    }(); ///< Native dependencies and the shared active-mode query required by send-keys.
} // namespace

std::span<const CopyModeCommandDescription> copy_mode_commands() { return command_catalog; }

std::span<const protocol::native::OperationId> copy_mode_command_operations() {
    return {required_operations.first.data(), required_operations.second};
}

Result<CopyModeCommandRequest> read_copy_mode_command(std::string_view name, PaneId pane,
                                                      std::uint32_t count) {
    const auto command =
        std::ranges::find(command_catalog, name, &CopyModeCommandDescription::name);
    if (command == command_catalog.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "unsupported copy-mode action"});
    }
    return std::visit(
        /** @brief Pass repeat prefixes only to counted motions; stock uncounted actions ignore
           them. */
        [pane, count](auto read) -> CopyModeCommandRequest {
            if constexpr (std::is_same_v<decltype(read), RepeatedCopyModeCommandReader>) {
                return read(pane, count);
            } else {
                return read(pane);
            }
        },
        command->read);
}

Result<void> execute_copy_mode_command(Server &server, const CopyModeCommandRequest &request) {
    return std::visit(
        /** @brief Execute the selected request's native operation without interpreting action
           names. */
        [&server](const auto &command) { return command.execute(server); }, request);
}

PaneId copy_mode_command_pane(const CopyModeCommandRequest &request) {
    return std::visit(
        /** @brief Borrow the concrete request's already parsed pane identity. */
        [](const auto &command) { return command.arguments.pane; }, request);
}
} // namespace tmux_cxx
