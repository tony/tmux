#include "window_link/operations/select_window_key_binding.hpp"

#include <tmux_cxx/window_link/operations/select_window.hpp>

namespace tmux_cxx {
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
                       return selected.link != invocation.selection.window_link ||
                                      selected.pane != invocation.selection.pane
                                  ? key_table::KeyBindingEffect::client_selection_changed
                                  : key_table::KeyBindingEffect::client_selection_unchanged;
                   });
}
} // namespace tmux_cxx
