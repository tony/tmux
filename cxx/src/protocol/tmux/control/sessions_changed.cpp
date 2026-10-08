#include "protocol/tmux/control/sessions_changed.hpp"

namespace tmux_cxx::protocol::tmux::control {
std::string format_sessions_changed() { return "%sessions-changed\n"; }
} // namespace tmux_cxx::protocol::tmux::control
