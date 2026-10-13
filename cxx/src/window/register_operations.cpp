#include "register_operations.hpp"

#include <tmux_cxx/window/operations/create_window.hpp>
#include <tmux_cxx/window/operations/read_window.hpp>
#include <tmux_cxx/window/operations/rename_window.hpp>
#include <tmux_cxx/window/operations/select_layout.hpp>

namespace tmux_cxx {
Result<void> register_window_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<ReadWindow>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<CreateWindow>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<RenameWindow>(); !registration) {
        return registration;
    }
    return registry.add<SelectLayout>();
}
} // namespace tmux_cxx
