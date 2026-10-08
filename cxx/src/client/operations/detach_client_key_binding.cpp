#include "client/operations/detach_client_key_binding.hpp"

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
DetachClientKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    if (auto detached = DetachClient::execute(invocation.server, {invocation.client}); !detached) {
        return std::unexpected(detached.error());
    }
    return key_table::KeyBindingEffect::client_detached;
}
} // namespace tmux_cxx
