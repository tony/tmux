#pragma once

#include <tmux_cxx/identity.hpp>

#include <vector>

namespace tmux_cxx {
class Server;

/** @brief Select and enqueue coalesced current-state redraws independently of terminal delivery. */
class ClientViewRefresh {
  public:
    /** @brief Queue invalidated client views and return clients whose current route failed. */
    static std::vector<ClientId> queue_invalidated(Server &server);
};
} // namespace tmux_cxx
