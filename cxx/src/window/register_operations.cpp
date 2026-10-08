#include "register_operations.hpp"

#include <tmux_cxx/window/operations/read_window.hpp>

namespace tmux_cxx {
Result<void> register_window_operations(protocol::native::OperationRegistry &registry) {
    return registry.add<ReadWindow>();
}
} // namespace tmux_cxx
