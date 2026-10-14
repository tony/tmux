#pragma once

#include "key_table/key_binding.hpp"
#include "pane/operations/copy_mode_command_catalog.hpp"

#include <tmux_cxx/pane/operations/enter_copy_mode.hpp>
#include <tmux_cxx/window/mode_keys.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-[ to frozen copy-mode entry for the invoking client's selected pane. */
struct EnterCopyModeKeyBinding {
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'['}, "copy-mode", EnterCopyMode::description.id};
    ///< Default stock entry key and shared native operation dependency.
    /** @brief Enter pane-owned mode and resolve mode routing before the next retained token. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};

/** @brief Bind one copy-table key to a typed stock action and its explicit routing consequence. */
template <WindowModeKeys Keys, key_table::KeyCode Key, class Command,
          key_table::KeyBindingEffect Effect =
              key_table::KeyBindingEffect::client_selection_unchanged>
struct CopyModeKeyBinding {
    static_assert(valid_window_mode_keys(Keys));
    static constexpr key_table::KeyBindingDescription description{
        Keys == WindowModeKeys::vi ? "copy-mode-vi" : "copy-mode", Key, Command::name,
        Command::Operation::description.id};
    ///< Persistent table identity and the action's required shared native operation.
    /** @brief Execute one native action and report routing changes only after success. */
    static Result<key_table::KeyBindingEffect>
    execute(key_table::KeyBindingInvocation &invocation) {
        if (auto result = Command::read(invocation.selection.pane).execute(invocation.server);
            !result) {
            return std::unexpected(result.error());
        }
        return Effect;
    }
};

/** @brief Bind mode cancellation with mandatory re-resolution of following retained keys. */
template <WindowModeKeys Keys, key_table::KeyCode Key>
using CancelCopyModeKeyBinding = CopyModeKeyBinding<Keys, Key, CancelCopyModeCommand,
                                                    key_table::KeyBindingEffect::pane_mode_changed>;

/** @brief Adapt stock copy-pipe keys to automatic server-buffer copying and mode cancellation. */
template <WindowModeKeys Keys, key_table::KeyCode Key>
using CopySelectionAndCancelKeyBinding =
    CopyModeKeyBinding<Keys, Key, CopySelectionCommand<CopySelectionDisposition::cancel_mode>,
                       key_table::KeyBindingEffect::pane_mode_changed>;
} // namespace tmux_cxx
