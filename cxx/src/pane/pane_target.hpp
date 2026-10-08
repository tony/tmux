#pragma once

#include <tmux_cxx/identity.hpp>

#include <optional>
#include <string_view>

namespace tmux_cxx {
/** @brief Parse an exact percent-prefixed pane identity without names or partial numbers. */
std::optional<PaneId> parse_pane_target(std::string_view target);
} // namespace tmux_cxx
