#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/client/operations/detach_client.hpp>

namespace tmux_cxx {
/** @brief Bind the default prefix-table detach key to the typed client operation. */
struct DetachClientKeyBinding {
    /** @brief Declare the tmux default prefix-d mapping and its operation dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{'d'}, "detach-client", DetachClient::description.id};
    /** @brief Detach the invoking client while preserving its persistent session. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &invocation);
};
} // namespace tmux_cxx
