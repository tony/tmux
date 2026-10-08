#pragma once

#include "protocol/native/operation_registry.hpp"

namespace tmux_cxx {
/** @brief Register typed window-membership operations before the native catalog freezes. */
Result<void> register_window_link_operations(protocol::native::OperationRegistry &registry);
} // namespace tmux_cxx
