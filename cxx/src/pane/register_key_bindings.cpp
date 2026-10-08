#include "pane/register_key_bindings.hpp"

#include "pane/operations/select_next_pane_key_binding.hpp"
#include "pane/operations/send_prefix_key_binding.hpp"
#include "pane/operations/split_pane_key_bindings.hpp"

namespace tmux_cxx {
Result<void> register_pane_key_bindings(key_table::KeyBindingRegistry &registry) {
    if (auto registration = registry.add<SendPrefixKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SplitPaneTopBottomKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SplitPaneLeftRightKeyBinding>(); !registration) {
        return registration;
    }
    return registry.add<SelectNextPaneKeyBinding>();
}
} // namespace tmux_cxx
