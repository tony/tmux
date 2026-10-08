#pragma once

#include "protocol/native/operation_registry.hpp"

namespace tmux_cxx {
/** @brief Register the server's typed session operations before the native catalog freezes. */
Result<void> register_session_operations(protocol::native::OperationRegistry &registry);
} // namespace tmux_cxx
