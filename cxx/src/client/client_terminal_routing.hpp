#pragma once

#include "key_table/key_table_catalog.hpp"

#include <tmux_cxx/identity.hpp>

#include "terminal/terminal_key_decoder.hpp"

#include <vector>

namespace tmux_cxx {
class Server;
/** @brief Describe borrowed terminal readiness routes for one attached ordinary tmux client. */
struct ClientTerminalRoute {
    ClientId client_id;        ///< Connection-lifetime client owning both descriptors.
    int input_descriptor;      ///< Nonblocking outer-terminal input descriptor.
    int output_descriptor;     ///< Nonblocking outer-terminal output descriptor.
    bool input_enabled;        ///< Bounded client input storage can accept another terminal read.
    bool output_pending;       ///< Composed view bytes require writable readiness.
    bool key_routing_deferred; ///< Queued keys can resume in another turn without descriptor I/O.
    std::optional<TerminalKeyDecoder::Clock::time_point> key_deadline; ///< Pending key readiness.
};

/** @brief Classify one captured route after concurrent lifecycle mutations in the event turn. */
enum class ClientTerminalRouteOutcome {
    routed,        ///< Ready input and output were transferred through the current attachment.
    superseded,    ///< A detach or disconnect invalidated this captured route.
    action_failed, ///< A consumed action failed; its queued message preserves the attachment.
    failed,        ///< The current client-to-session/window/pane route failed.
};

/** @brief Route attached-client terminal readiness without selecting or composing redraws. */
class ClientTerminalRouting {
  public:
    /** @brief Capture attached client descriptors until the next client lifecycle mutation. */
    static std::vector<ClientTerminalRoute> capture(const Server &server);
    /** @brief Route terminal I/O with one shared key allowance across target and capacity changes.
     */
    static ClientTerminalRouteOutcome
    route_ready_io(Server &server, ClientId client_id, const key_table::KeyTableCatalog &key_tables,
                   short input_events, short output_events,
                   TerminalKeyDecoder::Clock::time_point now = TerminalKeyDecoder::Clock::now());
};
} // namespace tmux_cxx
