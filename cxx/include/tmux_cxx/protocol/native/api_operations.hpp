#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>

#include <span>

namespace tmux_cxx::protocol::native {
/** @brief Borrow the immutable public operation contract for the process lifetime.
 *
 * Startup rejects a daemon registry that omits or changes these descriptions. */
std::span<const OperationDescription> api_operations();
} // namespace tmux_cxx::protocol::native
