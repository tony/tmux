#pragma once

#include <tmux_cxx/protocol/bytes.hpp>

#include <cstdint>
#include <string>

namespace tmux_cxx {
/** @brief Own one completed stock-server command's status and byte-preserving output streams. */
struct StockCommandResult {
    std::int32_t exit_status = 0;    ///< Host-ABI status returned by the stock server.
    protocol::Bytes standard_output; ///< Bytes delivered through stock stream descriptor 1.
    protocol::Bytes standard_error;  ///< Bytes delivered through stock stream descriptor 2.
    std::string exit_message;        ///< Optional stock exit explanation without its terminator.
};
} // namespace tmux_cxx
