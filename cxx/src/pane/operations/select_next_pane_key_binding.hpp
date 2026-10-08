#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/pane/operations/select_pane.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-o to the next pane in the selected window's layout order. */
struct SelectNextPaneKeyBinding {
    /** @brief Declare tmux's default prefix-o mapping and pane-selection dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'o'}, "select-pane", SelectPane::description.id};
    /** @brief Resolve the next pane at execution, then invoke the typed selection operation. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
