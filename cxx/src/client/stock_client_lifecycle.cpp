#include "client/stock_client_lifecycle.hpp"

#include "client/client.hpp"
#include "server/state.hpp"
#include <tmux_cxx/protocol/tmux/protocol_quirk.hpp>

namespace tmux_cxx {
Result<ClientId> StockClientLifecycle::accept(Server &server) {
    if (server.state_->shutdown_attempted) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "server is shutting down"});
    }
    const ClientId client_id{server.state_->next_client_identity++};
    const auto revision = ++server.state_->revision;
    server.state_->clients.emplace(client_id, std::make_unique<Client>(client_id, revision));
    return client_id;
}

Result<void> StockClientLifecycle::identify_terminal(
    Server &server, ClientId client_id, posix::OwnedFd input, posix::OwnedFd output,
    protocol::tmux::StockConnectionCompatibility compatibility) {
    const auto client_entry = server.state_->clients.find(client_id);
    if (client_entry == server.state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    if (client_entry->second->stock_identification_complete) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "client identification is already complete"});
    }
    auto client_terminal = ClientTerminal::adopt(std::move(input), std::move(output));
    if (!client_terminal) {
        return std::unexpected(client_terminal.error());
    }
    if ((*client_terminal)->size_origin() == ClientTerminalSizeOrigin::zero_dimension_defaults &&
        !protocol::tmux::activate_protocol_quirk(
            compatibility, protocol::tmux::ProtocolQuirk::zero_terminal_dimensions_use_defaults)) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "connection policy rejects zero terminal dimensions"});
    }
    client_entry->second->terminal = std::move(*client_terminal);
    client_entry->second->stock_compatibility = std::move(compatibility);
    client_entry->second->stock_identification_complete = true;
    client_entry->second->revision = ++server.state_->revision;
    return {};
}

Result<void>
StockClientLifecycle::identify_command(Server &server, ClientId client_id,
                                       protocol::tmux::StockConnectionCompatibility compatibility) {
    const auto client_entry = server.state_->clients.find(client_id);
    if (client_entry == server.state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    if (client_entry->second->stock_identification_complete) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "client identification is already complete"});
    }
    client_entry->second->stock_compatibility = std::move(compatibility);
    client_entry->second->stock_identification_complete = true;
    client_entry->second->revision = ++server.state_->revision;
    return {};
}

Result<void> StockClientLifecycle::update_stock_compatibility(
    Server &server, ClientId client_id,
    protocol::tmux::StockConnectionCompatibility compatibility) {
    const auto client_entry = server.state_->clients.find(client_id);
    if (client_entry == server.state_->clients.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "client no longer exists"});
    }
    if (!client_entry->second->stock_identification_complete ||
        !client_entry->second->stock_compatibility) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "stock client identification is incomplete"});
    }
    client_entry->second->stock_compatibility = std::move(compatibility);
    client_entry->second->revision = ++server.state_->revision;
    return {};
}

void StockClientLifecycle::disconnect(Server &server, ClientId client_id) {
    const auto client_entry = server.state_->clients.find(client_id);
    if (client_entry == server.state_->clients.end()) {
        return;
    }
    server.state_->clients.erase(client_entry);
    ++server.state_->revision;
}
} // namespace tmux_cxx
