#pragma once

#include "protocol/native/operation_registry.hpp"

namespace tmux_cxx {
/** @brief Add every paste-buffer operation to a mutable native catalog. */
Result<void> register_paste_buffer_operations(protocol::native::OperationRegistry &registry);
} // namespace tmux_cxx
