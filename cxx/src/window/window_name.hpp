#pragma once

#include <string_view>

namespace tmux_cxx {
/** @brief Accept nonempty UTF-8 window names up to 128 bytes without ASCII controls or DEL. */
bool valid_window_name(std::string_view name);
} // namespace tmux_cxx
