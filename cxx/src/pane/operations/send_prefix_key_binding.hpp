#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/pane/operations/send_pane_input.hpp>

namespace tmux_cxx {
/** @brief Bind C-b in the prefix table to send the session's primary prefix to its selected pane.
 */
struct SendPrefixKeyBinding {
    /** @brief Declare tmux's C-b send-prefix mapping and pane-input dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::control_key('b'), "send-prefix", SendPaneInput::description.id};
    /** @brief Admit the effective primary prefix to the pane selected for this binding. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
