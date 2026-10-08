#pragma once

#include <tmux_cxx/tmux_error.hpp>

#include <cerrno>
#include <cstring>

/** @namespace tmux_cxx::posix
 * @brief Small ownership and error adapters for direct POSIX resource use.
 */
namespace tmux_cxx::posix {
/** @brief Preserve errno context from a failed native resource operation. */
inline TmuxError errno_error(std::string_view action) {
    const auto error_number = errno;
    return {TmuxErrorCode::io, std::string(action) + ": " + std::strerror(error_number)};
}
} // namespace tmux_cxx::posix
