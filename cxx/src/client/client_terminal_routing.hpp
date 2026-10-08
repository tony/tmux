#pragma once

#include "key_table/key_table_catalog.hpp"

#include <tmux_cxx/identity.hpp>

#include <vector>

namespace tmux_cxx {
class Server;
/** @brief Describe borrowed terminal readiness routes for one attached ordinary tmux client. */
struct ClientTerminalRoute {
    ClientId client_id;    ///< Connection-lifetime client owning both descriptors.
    int input_descriptor;  ///< Nonblocking outer-terminal input descriptor.
    int output_descriptor; ///< Nonblocking outer-terminal output descriptor.
    bool input_enabled;    ///< Bounded client input storage can accept another terminal read.
    bool output_pending;   ///< Composed view bytes require writable readiness.
};

/** @brief Classify one captured route after concurrent lifecycle mutations in the event turn. */
enum class ClientTerminalRouteOutcome {
    routed,     ///< Ready input and output were transferred through the current attachment.
    superseded, ///< A detach or disconnect invalidated this captured route.
    failed,     ///< The current client-to-session/window/pane route failed.
};

/** @brief Route attached-client terminal readiness without selecting or composing redraws. */
class ClientTerminalRouting {
  public:
    /** @brief Capture attached client descriptors until the next client lifecycle mutation. */
    static std::vector<ClientTerminalRoute> capture(const Server &server);
    /** @brief Route ready terminal input and output through the client's active pane. */
    static ClientTerminalRouteOutcome route_ready_io(Server &server, ClientId client_id,
                                                     const key_table::KeyTableCatalog &key_tables,
                                                     short input_events, short output_events);
};
} // namespace tmux_cxx
