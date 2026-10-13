#include "window_link/operations/select_window_index_key_binding.hpp"
#include "window_link/operations/select_window_key_binding.hpp"

#include <tmux_cxx/server/server.hpp>

#include <algorithm>

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
select_window_index_for_key(key_table::KeyBindingInvocation &invocation, std::uint32_t index) {
    const auto links = invocation.server.window_links(invocation.selection.session);
    if (!links) {
        return std::unexpected(links.error());
    }
    const auto target = std::ranges::find(*links, index, &WindowLinkSnapshot::index);
    if (target == links->end()) {
        return key_table::KeyBindingEffect::client_selection_unchanged;
    }
    return select_window_link_for_key(invocation, *target);
}
} // namespace tmux_cxx
