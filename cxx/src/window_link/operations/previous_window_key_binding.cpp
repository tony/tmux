#include "window_link/operations/previous_window_key_binding.hpp"
#include "window_link/operations/select_window_key_binding.hpp"

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
PreviousWindowKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    return PreviousWindow::execute(invocation.server, {invocation.selection.session})
        .transform(/** @copybrief tmux_cxx::window_selection_key_effect */
                   [&invocation](const SessionSnapshot &selected) {
                       return window_selection_key_effect(invocation.selection, selected);
                   });
}
} // namespace tmux_cxx
