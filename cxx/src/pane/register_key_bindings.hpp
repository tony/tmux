#pragma once

#include "key_table/key_binding_registry.hpp"

namespace tmux_cxx {
/** @brief Register pane-owned default key bindings during server catalog construction. */
Result<void> register_pane_key_bindings(key_table::KeyBindingRegistry &registry);
} // namespace tmux_cxx
