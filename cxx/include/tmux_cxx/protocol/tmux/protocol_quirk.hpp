#pragma once

#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>

#include <cstdint>
#include <string_view>

namespace tmux_cxx::protocol::tmux {
/** @brief Identify a declared compatibility adjustment independently of its active report text. */
enum class ProtocolQuirk : std::uint8_t {
    zero_terminal_dimensions_use_defaults,
    unattached_control_channel_stays_open,
};

/** @brief Declare where one adjustment applies and the fidelity change users can observe. */
struct ProtocolQuirkDescription {
    ProtocolQuirk id;            ///< Stable typed identity used by the adapter.
    std::string_view name;       ///< Stable public spelling exposed in compatibility reports.
    ProtocolDirection direction; ///< Adapter role permitted to activate this adjustment.
    std::uint8_t modes;         ///< Bit set of connection modes where this adjustment is permitted.
    std::string_view rationale; ///< Constraint that requires the adjustment.
    std::string_view consequence; ///< User-visible behavior changed by the adjustment.
    bool changes_fidelity; ///< True when strict compatibility may need to reject this adjustment.
};

/** @brief Return the declaration for a known compatibility adjustment. */
constexpr ProtocolQuirkDescription protocol_quirk(ProtocolQuirk quirk) {
    switch (quirk) {
    case ProtocolQuirk::zero_terminal_dimensions_use_defaults:
        return {
            quirk,
            "zero_terminal_dimensions_use_defaults",
            ProtocolDirection::stock_client_to_cxx_server,
            static_cast<std::uint8_t>((1U << static_cast<unsigned>(ConnectionMode::command)) |
                                      (1U << static_cast<unsigned>(ConnectionMode::interactive))),
            "the terminal engine requires nonzero bounded cell dimensions",
            "zero rows use 24 and zero columns use 80",
            true};
    case ProtocolQuirk::unattached_control_channel_stays_open:
        return {quirk,
                "unattached_control_channel_stays_open",
                ProtocolDirection::stock_client_to_cxx_server,
                static_cast<std::uint8_t>(1U << static_cast<unsigned>(ConnectionMode::control)),
                "the limited control subset does not yet attach a client to a session",
                "an unattached control client may submit commands after its initial command",
                true};
    }
    return {quirk,     "unknown", ProtocolDirection::stock_client_to_cxx_server, 0, "unknown",
            "unknown", true};
}

/** @brief Activate one policy-permitted adjustment and expose its observable consequence once. */
bool activate_protocol_quirk(StockConnectionCompatibility &compatibility, ProtocolQuirk quirk);

/** @brief Reapply typed active adjustments when a connection changes command or interactive mode.
 */
void retain_protocol_quirks(const StockConnectionCompatibility &previous,
                            StockConnectionCompatibility &replacement);
} // namespace tmux_cxx::protocol::tmux
