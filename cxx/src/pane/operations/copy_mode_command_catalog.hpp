#pragma once

#include <tmux_cxx/pane/operations/begin_copy_selection.hpp>
#include <tmux_cxx/pane/operations/cancel_copy_mode.hpp>
#include <tmux_cxx/pane/operations/clear_copy_selection.hpp>
#include <tmux_cxx/pane/operations/copy_selection.hpp>
#include <tmux_cxx/pane/operations/move_copy_cursor.hpp>
#include <tmux_cxx/pane/operations/move_copy_cursor_to.hpp>
#include <tmux_cxx/pane/operations/scroll_copy_mode.hpp>

#include <array>
#include <span>
#include <utility>
#include <variant>

namespace tmux_cxx {
/** @brief Own one copy command's concrete native request until typed execution. */
template <class Operation> struct CopyModeCommandInvocation {
    typename Operation::Request arguments; ///< Owned pane target and operation-specific arguments.
    /** @brief Execute the shared operation and discard only command-unexposed success metadata. */
    Result<void> execute(Server &server) const {
        const auto result = Operation::execute(server, arguments);
        if (!result) {
            return std::unexpected(result.error());
        }
        return {};
    }
};

/** @brief Construct counted relative cursor commands from closed native directions. */
template <CopyCursorDirection Direction> struct CopyCursorCommand {
    static_assert(valid_copy_cursor_direction(Direction));
    using Operation = MoveCopyCursor; ///< Shared native cursor mutation.
    /** @brief Stock spellings indexed by their closed native direction. */
    static constexpr std::array<std::string_view, 4> names{"cursor-left", "cursor-right",
                                                           "cursor-up", "cursor-down"};
    static constexpr std::string_view name = names[std::to_underlying(Direction)];
    ///< Exact stock action spelling.
    /** @brief Own relative steps without interpreting window key conventions in the adapter. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane, std::uint32_t count = 1) {
        return {{pane, Direction, count}};
    }
};

/** @brief Construct uncounted logical-line and history boundary commands. */
template <CopyCursorDestination Destination> struct CopyCursorDestinationCommand {
    static_assert(valid_copy_cursor_destination(Destination));
    using Operation = MoveCopyCursorTo; ///< Shared native boundary mutation.
    /** @brief Stock spellings indexed by their closed native destination. */
    static constexpr std::array<std::string_view, 4> names{"start-of-line", "end-of-line",
                                                           "history-top", "history-bottom"};
    static constexpr std::string_view name = names[std::to_underlying(Destination)];
    ///< Exact stock action spelling.
    /** @brief Own a boundary destination without inventing a repeat-count parameter. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane) { return {{pane, Destination}}; }
};

/** @brief Construct counted frozen-viewport scroll commands from closed native motions. */
template <CopyScrollMotion Motion> struct CopyScrollCommand {
    static_assert(valid_copy_scroll_motion(Motion));
    using Operation = ScrollCopyMode; ///< Shared native viewport mutation.
    /** @brief Stock spellings indexed by their closed native scroll motion. */
    static constexpr std::array<std::string_view, 6> names{
        "scroll-up", "scroll-down", "halfpage-up", "halfpage-down", "page-up", "page-down"};
    static constexpr std::string_view name = names[std::to_underlying(Motion)];
    ///< Exact stock action spelling.
    /** @brief Own scroll steps while leaving viewport and cursor semantics in the shared core. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane, std::uint32_t count = 1) {
        return {{pane, Motion, count}};
    }
};

/** @brief Construct automatic-buffer copy commands with an explicit post-storage disposition. */
template <CopySelectionDisposition Disposition> struct CopySelectionCommand {
    static_assert(valid_copy_selection_disposition(Disposition));
    using Operation = CopySelection; ///< Shared native extraction and automatic-buffer storage.
    /** @brief Stock spellings indexed by their closed native post-copy disposition. */
    static constexpr std::array<std::string_view, 3> names{
        "copy-selection-no-clear", "copy-selection", "copy-selection-and-cancel"};
    static constexpr std::string_view name = names[std::to_underlying(Disposition)];
    ///< Exact stock action spelling.
    /** @brief Own the storage disposition without introducing copy-pipe or clipboard effects. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane) { return {{pane, Disposition}}; }
};

/** @brief Construct character-selection entry at the current frozen cursor. */
struct BeginCopySelectionCommand {
    using Operation = BeginCopySelection; ///< Shared native selection mutation.
    static constexpr std::string_view name = "begin-selection"; ///< Exact stock action spelling.
    /** @brief Own one selection target without reading or copying terminal state in the adapter. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane) { return {{pane}}; }
};

/** @brief Construct character-selection clearing while retaining frozen mode state. */
struct ClearCopySelectionCommand {
    using Operation = ClearCopySelection; ///< Shared native selection mutation.
    static constexpr std::string_view name = "clear-selection"; ///< Exact stock action spelling.
    /** @brief Own one selection target while leaving current-key cursor clipping to the shared
     * operation. */
    static CopyModeCommandInvocation<Operation> read(PaneId pane) { return {{pane}}; }
};

/** @brief Construct explicit cancellation of the addressed pane's copy mode. */
struct CancelCopyModeCommand {
    using Operation = CancelCopyMode;                  ///< Shared native mode cancellation.
    static constexpr std::string_view name = "cancel"; ///< Exact stock action spelling.
    /** @brief Own one cancellation target while leaving inactive-command validation to send-keys.
     */
    static CopyModeCommandInvocation<Operation> read(PaneId pane) { return {{pane}}; }
};

/** @brief Preserve a closed set of typed copy requests after stock argument parsing. */
using CopyModeCommandRequest = std::variant<
    CopyModeCommandInvocation<MoveCopyCursor>, CopyModeCommandInvocation<MoveCopyCursorTo>,
    CopyModeCommandInvocation<ScrollCopyMode>, CopyModeCommandInvocation<BeginCopySelection>,
    CopyModeCommandInvocation<ClearCopySelection>, CopyModeCommandInvocation<CopySelection>,
    CopyModeCommandInvocation<CancelCopyMode>>;
/** @brief Build a boundary or selection request without passing an ignored repeat count. */
using CopyModeCommandReader = CopyModeCommandRequest (*)(PaneId);
/** @brief Build a repeated cursor or viewport request with its validated stock prefix count. */
using RepeatedCopyModeCommandReader = CopyModeCommandRequest (*)(PaneId, std::uint32_t);
/** @brief Declare one exact stock action, its required native operation, and typed argument shape.
 */
struct CopyModeCommandDescription {
    std::string_view name;                   ///< Exact supported send-keys -X action spelling.
    protocol::native::OperationId operation; ///< Required shared native operation.
    std::variant<CopyModeCommandReader, RepeatedCopyModeCommandReader> read;
    ///< Argument shape preserves the distinction between repeated motions and uncounted actions.
};

/** @brief Borrow the immutable catalog of supported stock copy-mode actions. */
std::span<const CopyModeCommandDescription> copy_mode_commands();
/** @brief Borrow unique native dependencies derived from every cataloged action and active-mode
 * query. */
std::span<const protocol::native::OperationId> copy_mode_command_operations();
/** @brief Resolve exact action spelling and apply repeat counts only to source-defined repeated
 * motions. */
Result<CopyModeCommandRequest> read_copy_mode_command(std::string_view name, PaneId pane,
                                                      std::uint32_t count);
/** @brief Execute the selected typed copy request without dispatching on action strings or numbers.
 */
Result<void> execute_copy_mode_command(Server &server, const CopyModeCommandRequest &request);
/** @brief Read the owned pane target from the selected concrete copy request. */
PaneId copy_mode_command_pane(const CopyModeCommandRequest &request);
} // namespace tmux_cxx
