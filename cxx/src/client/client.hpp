#pragma once

#include "client/client_key_input.hpp"
#include "client/client_terminal.hpp"
#include "client/client_view.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>

#include <memory>
#include <optional>
#include <string>

namespace tmux_cxx {
/** @brief Own one tmux client's attachment and terminal route independently of retained sessions.
 */
struct Client {
    /** @brief Create one unidentified connection-lifetime client at its admission revision. */
    Client(ClientId identity, std::uint64_t admitted_revision)
        : id(identity), revision(admitted_revision) {}
    ClientId id;                      ///< Stable identity for this connection lifetime.
    std::optional<SessionId> session; ///< Currently attached retained session.
    std::optional<SessionId>
        previous_session;      ///< Prior session retained only across direct retargeting.
    std::string detached_from; ///< Session name reported during the latest detachment.
    std::optional<protocol::tmux::StockConnectionCompatibility>
        stock_compatibility; ///< Selected stock profile, mode support, evidence, and limitations.
    std::unique_ptr<ClientTerminal>
        terminal;             ///< Ordinary terminal route, absent for command-mode clients.
    ClientKeyInput key_input; ///< Active table and bounded bytes awaiting input routing.
    ClientView view; ///< Composed current-state view independent of the terminal delivery route.
    bool stock_identification_complete =
        false;              ///< Stock identification completed before command execution.
    std::uint64_t revision; ///< Most recent client mutation in the server revision sequence.
};
} // namespace tmux_cxx
