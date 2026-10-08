#include "protocol/tmux/control/pane_output.hpp"

namespace tmux_cxx::protocol::tmux::control {
std::string format_pane_output(PaneId pane, std::string_view bytes) {
    std::string record = "%output %" + std::to_string(pane.value) + " ";
    record.reserve(record.size() + bytes.size() * 4U + 1U);
    for (const unsigned char byte : bytes) {
        if (byte < ' ' || byte == '\\') {
            record.push_back('\\');
            record.push_back(static_cast<char>('0' + ((byte >> 6U) & 7U)));
            record.push_back(static_cast<char>('0' + ((byte >> 3U) & 7U)));
            record.push_back(static_cast<char>('0' + (byte & 7U)));
        } else {
            record.push_back(static_cast<char>(byte));
        }
    }
    record.push_back('\n');
    return record;
}
} // namespace tmux_cxx::protocol::tmux::control
