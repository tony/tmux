#include "pane/operations/select_next_pane_key_binding.hpp"

#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <iterator>

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
SelectNextPaneKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    auto panes = invocation.server.panes(invocation.window);
    if (!panes) {
        return std::unexpected(panes.error());
    }
    const auto active = std::ranges::find(*panes, true, &PaneSnapshot::active);
    if (active == panes->end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "selected window has no active pane"});
    }
    auto next = std::next(active);
    if (next == panes->end()) {
        next = panes->begin();
    }
    const bool selected_pane_changed = next->id != active->id;
    auto selected = SelectPane::execute(invocation.server, {next->id});
    if (!selected) {
        return std::unexpected(selected.error());
    }
    return selected_pane_changed ? key_table::KeyBindingEffect::selected_pane_changed
                                 : key_table::KeyBindingEffect::selected_pane_unchanged;
}
} // namespace tmux_cxx
