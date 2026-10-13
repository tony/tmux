#include "window_link/operations/select_window_key_binding.hpp"

#include <tmux_cxx/window_link/operations/select_window.hpp>

namespace tmux_cxx {
key_table::KeyBindingEffect window_selection_key_effect(const ClientSelection &before,
                                                        const SessionSnapshot &selected) {
    return selected.link != before.window_link || selected.pane != before.pane
               ? key_table::KeyBindingEffect::client_selection_changed
               : key_table::KeyBindingEffect::client_selection_unchanged;
}
Result<key_table::KeyBindingEffect>
select_window_link_for_key(key_table::KeyBindingInvocation &invocation,
                           const WindowLinkSnapshot &window_link) {
    if (window_link.session != invocation.selection.session) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window membership belongs to another session"});
    }
    return SelectWindow::execute(invocation.server, {window_link.id})
        .transform(/** @brief Request fresh input routing when session membership or pane changes.
                    */
                   [&invocation](const SessionSnapshot &selected) {
                       return window_selection_key_effect(invocation.selection, selected);
                   });
}
} // namespace tmux_cxx
