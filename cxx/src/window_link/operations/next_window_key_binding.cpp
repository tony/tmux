#include "window_link/operations/next_window_key_binding.hpp"
#include "window_link/operations/select_window_key_binding.hpp"

#include <tmux_cxx/server/server.hpp>

#include <algorithm>
#include <iterator>

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
NextWindowKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    const auto links = invocation.server.window_links(invocation.selection.session);
    if (!links) {
        return std::unexpected(links.error());
    }
    const auto current =
        std::ranges::find(*links, invocation.selection.window_link, &WindowLinkSnapshot::id);
    if (current == links->end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "client's selected window membership is stale"});
    }
    auto next = std::next(current);
    if (next == links->end()) {
        next = links->begin();
    }
    return select_window_link_for_key(invocation, *next);
}
} // namespace tmux_cxx
