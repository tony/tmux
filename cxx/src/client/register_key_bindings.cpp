#include "client/register_key_bindings.hpp"

#include "client/operations/detach_client_key_binding.hpp"

namespace tmux_cxx {
Result<void> register_client_key_bindings(key_table::KeyBindingRegistry &registry) {
    return registry.add<DetachClientKeyBinding>();
}
} // namespace tmux_cxx
