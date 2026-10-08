#include "register_operations.hpp"

#include <tmux_cxx/window_link/operations/link_window.hpp>
#include <tmux_cxx/window_link/operations/list_window_links.hpp>
#include <tmux_cxx/window_link/operations/unlink_window.hpp>

namespace tmux_cxx {
Result<void> register_window_link_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<LinkWindow>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<UnlinkWindow>(); !registration) {
        return registration;
    }
    return registry.add<ListWindowLinks>();
}
} // namespace tmux_cxx
