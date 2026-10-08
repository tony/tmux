#pragma once

#include <cstdint>
#include <vector>

namespace tmux_cxx::protocol {
/** @brief Own uninterpreted wire bytes without assigning them to a protocol or text encoding. */
using Bytes = std::vector<std::uint8_t>;
} // namespace tmux_cxx::protocol
