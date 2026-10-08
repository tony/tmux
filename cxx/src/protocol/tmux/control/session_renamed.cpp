#include "protocol/tmux/control/session_renamed.hpp"

namespace tmux_cxx::protocol::tmux::control {
std::string format_session_renamed(const SessionRenamedEvent &event) {
    return "%session-renamed $" + std::to_string(event.session.value) + " " + event.name + "\n";
}
} // namespace tmux_cxx::protocol::tmux::control
