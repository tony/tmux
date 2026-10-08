#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/pane/operations/split_pane.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-table percent to split the selected pane into left and right panes. */
struct SplitPaneLeftRightKeyBinding {
    /** @brief Declare tmux's default horizontal split mapping and operation dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'%'}, "split-window", SplitPane::description.id};
    /** @brief Split the selected pane left to right and select the new pane. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};

/** @brief Bind prefix-table quote to split the selected pane into top and bottom panes. */
struct SplitPaneTopBottomKeyBinding {
    /** @brief Declare tmux's default vertical split mapping and operation dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'"'}, "split-window", SplitPane::description.id};
    /** @brief Split the selected pane top to bottom and select the new pane. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
