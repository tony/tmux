#pragma once

#include "key_table/key_binding.hpp"
#include <tmux_cxx/window_link/operations/select_window.hpp>

namespace tmux_cxx {
/** @brief Select an exact session index or preserve selection when that index is absent. */
Result<key_table::KeyBindingEffect>
select_window_index_for_key(key_table::KeyBindingInvocation &invocation, std::uint32_t index);

/** @brief Bind one default prefix digit to its exact session-owned window index. */
template <std::uint32_t Index>
    requires(Index < 10)
struct SelectWindowIndexKeyBinding {
    /** @brief Declare one compile-time digit mapping and its typed window-selection dependency. */
    static constexpr key_table::KeyBindingDescription description{
        "prefix", key_table::KeyCode{static_cast<std::uint32_t>('0') + Index}, "select-window",
        SelectWindow::description.id};
    /** @brief Resolve this digit's index against the invoking client's current session. */
    static Result<key_table::KeyBindingEffect>
    execute(key_table::KeyBindingInvocation &invocation) {
        return select_window_index_for_key(invocation, Index);
    }
};
} // namespace tmux_cxx
