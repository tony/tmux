#pragma once

#include <cstddef>

namespace tmux_cxx::protocol::tmux::control {
inline constexpr std::size_t maximum_line_bytes = 64U * 1024U; ///< Retained command-line bound.
inline constexpr std::size_t maximum_arguments = 1000;         ///< Parsed arguments in one command.
inline constexpr std::size_t maximum_commands_per_turn = 128;  ///< Commands admitted per readiness.
inline constexpr std::size_t maximum_output_bytes = 1024U * 1024U; ///< Queued channel output.
inline constexpr std::size_t read_bytes_per_turn = 64U * 1024U;    ///< Input fairness allowance.
inline constexpr std::size_t write_bytes_per_turn = 64U * 1024U;   ///< Output fairness allowance.
inline constexpr std::size_t pane_output_bytes_per_turn =
    64U * 1024U; ///< Raw pane bytes encoded per control-client update turn.
} // namespace tmux_cxx::protocol::tmux::control
