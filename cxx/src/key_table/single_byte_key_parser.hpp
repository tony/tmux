#pragma once

#include <tmux_cxx/key_table/key_code.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <optional>
#include <string_view>

namespace tmux_cxx::key_table {
/** @brief Parse None, one ASCII byte, or a control letter into an optional key. */
Result<std::optional<KeyCode>> parse_single_byte_key(std::string_view text);
} // namespace tmux_cxx::key_table
