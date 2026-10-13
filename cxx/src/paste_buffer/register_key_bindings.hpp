#pragma once

#include "key_table/key_binding_registry.hpp"

namespace tmux_cxx {
/** @brief Register paste-buffer default keys before the server catalog freezes. */
Result<void> register_paste_buffer_key_bindings(key_table::KeyBindingRegistry &registry);
} // namespace tmux_cxx
