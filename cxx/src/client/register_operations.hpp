#pragma once

#include "protocol/native/operation_registry.hpp"

namespace tmux_cxx {
/** @brief Register typed client operations before the native catalog freezes. */
Result<void> register_client_operations(protocol::native::OperationRegistry &registry);
} // namespace tmux_cxx
