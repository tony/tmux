#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/window_link/operations/select_window.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-n to the invoking session's next window membership by index. */
struct NextWindowKeyBinding {
    /** @brief Declare tmux's next-window key and its typed selection dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'n'}, "next-window", SelectWindow::description.id};
    /** @brief Resolve the next live session membership, wrapping from the final index. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
