#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/session/session_snapshot.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

namespace tmux_cxx {
/** @brief Request fresh input routing when membership or the selected pane changes. */
key_table::KeyBindingEffect window_selection_key_effect(const ClientSelection &before,
                                                        const SessionSnapshot &selected);
/** @brief Select the invoking session's resolved membership and report its new input selection. */
Result<key_table::KeyBindingEffect>
select_window_link_for_key(key_table::KeyBindingInvocation &invocation,
                           const WindowLinkSnapshot &window_link);
} // namespace tmux_cxx
