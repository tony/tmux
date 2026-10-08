#include "pane/operations/split_pane_key_bindings.hpp"

namespace tmux_cxx {
namespace {
/** @brief Split the invocation's selected pane with the requested tmux layout orientation. */
Result<key_table::KeyBindingEffect> split_selected_pane(key_table::KeyBindingInvocation &invocation,
                                                        PaneSplitOrientation orientation) {
    if (auto split =
            SplitPane::execute(invocation.server, {invocation.pane, orientation, "/bin/sh"});
        !split) {
        return std::unexpected(split.error());
    }
    return key_table::KeyBindingEffect::selected_pane_changed;
}
} // namespace

Result<key_table::KeyBindingEffect>
SplitPaneLeftRightKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    return split_selected_pane(invocation, PaneSplitOrientation::left_right);
}

Result<key_table::KeyBindingEffect>
SplitPaneTopBottomKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    return split_selected_pane(invocation, PaneSplitOrientation::top_bottom);
}
} // namespace tmux_cxx
