#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/paste_buffer/operations/paste_top_buffer.hpp>

namespace tmux_cxx {
/** @brief Bind prefix-] to the selected pane's default automatic paste operation. */
struct PasteTopBufferKeyBinding {
    /** @brief Declare the default paste key and its typed operation dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{']'}, "paste-buffer", PasteTopBuffer::description.id};
    /** @brief Resolve default paste through shared pane admission without changing selection. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
