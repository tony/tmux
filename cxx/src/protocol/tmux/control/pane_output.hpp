#pragma once

#include <tmux_cxx/identity.hpp>

#include <string>
#include <string_view>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Encode raw pane bytes as one newline-terminated stock control output record. */
std::string format_pane_output(PaneId pane, std::string_view bytes);
} // namespace tmux_cxx::protocol::tmux::control
