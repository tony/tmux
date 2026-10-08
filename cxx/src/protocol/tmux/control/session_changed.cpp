#include "protocol/tmux/control/session_changed.hpp"

namespace tmux_cxx::protocol::tmux::control {
std::string format_session_changed(const SessionSnapshot &session) {
    return "%session-changed $" + std::to_string(session.id.value) + " " + session.name + "\n";
}
} // namespace tmux_cxx::protocol::tmux::control
