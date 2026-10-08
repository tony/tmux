#include <tmux_cxx/protocol/tmux/protocol_quirk.hpp>

#include <algorithm>

namespace tmux_cxx::protocol::tmux {
bool activate_protocol_quirk(StockConnectionCompatibility &compatibility, ProtocolQuirk quirk) {
    const auto declaration = protocol_quirk(quirk);
    const auto mode = static_cast<unsigned>(compatibility.mode);
    if (compatibility.direction != declaration.direction || mode >= 8 ||
        (declaration.modes & (1U << mode)) == 0) {
        return false;
    }
    if (!std::ranges::contains(compatibility.quirks, declaration.name)) {
        compatibility.quirks.emplace_back(declaration.name);
        compatibility.limitations.emplace_back(declaration.consequence);
    }
    return true;
}

void retain_protocol_quirks(const StockConnectionCompatibility &previous,
                            StockConnectionCompatibility &replacement) {
    for (const auto quirk : {ProtocolQuirk::zero_terminal_dimensions_use_defaults,
                             ProtocolQuirk::unattached_control_channel_stays_open}) {
        const auto declaration = protocol_quirk(quirk);
        if (std::ranges::contains(previous.quirks, declaration.name)) {
            (void)activate_protocol_quirk(replacement, quirk);
        }
    }
}
} // namespace tmux_cxx::protocol::tmux
