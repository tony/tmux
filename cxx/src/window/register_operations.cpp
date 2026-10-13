#include "register_operations.hpp"

#include <tmux_cxx/window/operations/create_window.hpp>
#include <tmux_cxx/window/operations/read_window.hpp>

namespace tmux_cxx {
Result<void> register_window_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<ReadWindow>(); !registration) {
        return registration;
    }
    return registry.add<CreateWindow>();
}
} // namespace tmux_cxx
