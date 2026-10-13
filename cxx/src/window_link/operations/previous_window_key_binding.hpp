#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/window_link/operations/select_window.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-p to the invoking session's previous window membership by index. */
struct PreviousWindowKeyBinding {
    /** @brief Declare tmux's previous-window key and its typed selection dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'p'}, "previous-window", SelectWindow::description.id};
    /** @brief Resolve the previous live session membership, wrapping from the first index. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
