#include "window_link/register_key_bindings.hpp"

#include "window_link/operations/next_window_key_binding.hpp"
#include "window_link/operations/previous_window_key_binding.hpp"
#include "window_link/operations/select_window_index_key_binding.hpp"

namespace tmux_cxx {
Result<void> register_window_link_key_bindings(key_table::KeyBindingRegistry &registry) {
    if (auto registration = registry.add<NextWindowKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<PreviousWindowKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<0>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<1>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<2>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<3>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<4>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<5>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<6>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<7>>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectWindowIndexKeyBinding<8>>(); !registration) {
        return registration;
    }
    return registry.add<SelectWindowIndexKeyBinding<9>>();
}
} // namespace tmux_cxx
