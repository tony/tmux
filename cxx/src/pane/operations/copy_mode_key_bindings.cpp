#include "pane/operations/copy_mode_key_bindings.hpp"

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
EnterCopyModeKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    if (auto entered = EnterCopyMode::execute(invocation.server, {invocation.selection.pane});
        !entered) {
        return std::unexpected(entered.error());
    }
    return key_table::KeyBindingEffect::pane_mode_changed;
}
} // namespace tmux_cxx
