#pragma once

#include "platform/posix/owned_fd.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/tmux_error.hpp>

namespace tmux_cxx {
class Server;
/** @brief Translate accepted stock-connection lifetime into server-owned tmux client entities. */
class StockClientLifecycle {
  public:
    /** @brief Create an unidentified client for one accepted stock transport. */
    static Result<ClientId> accept(Server &server);
    /** @brief Complete identification by adopting validated ordinary-terminal routes. */
    static Result<void>
    identify_terminal(Server &server, ClientId client_id, posix::OwnedFd input,
                      posix::OwnedFd output,
                      protocol::tmux::StockConnectionCompatibility compatibility);
    /** @brief Complete identification for a command client without an attachable terminal. */
    static Result<void>
    identify_command(Server &server, ClientId client_id,
                     protocol::tmux::StockConnectionCompatibility compatibility);
    /** @brief Replace one identified stock client's mode report after a lifecycle transition. */
    static Result<void>
    update_stock_compatibility(Server &server, ClientId client_id,
                               protocol::tmux::StockConnectionCompatibility compatibility);
    /** @brief End one client relationship and restore its terminal without destroying sessions. */
    static void disconnect(Server &server, ClientId client_id);
};
} // namespace tmux_cxx
