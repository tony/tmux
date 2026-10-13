#pragma once

#include "key_table/key_binding_registry.hpp"

namespace tmux_cxx {
/** @brief Register default next, previous, and indexed window keys with their typed dependency. */
Result<void> register_window_link_key_bindings(key_table::KeyBindingRegistry &registry);
} // namespace tmux_cxx
