#include "connection/stock_server_operation_registry.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx::detail {
Result<void> StockServerOperationRegistry::add_description(
    protocol::tmux::StockServerOperationDescription description) {
    if (registration_closed_ || description.name.empty() || description.command.empty() ||
        description.mode != protocol::tmux::ConnectionMode::command) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "invalid or late stock-server operation registration"});
    }
    for (const auto &registration : registrations_) {
        if (registration.name == description.name || registration.command == description.command) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument,
                          "duplicate stock-server operation identity or command"});
        }
    }
    registrations_.push_back(description);
    return {};
}

Result<StockServerOperationCatalog> StockServerOperationRegistry::freeze() && {
    if (registration_closed_ || registrations_.empty()) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "stock-server operation registry is empty or closed"});
    }
    std::ranges::sort(registrations_, {}, &protocol::tmux::StockServerOperationDescription::name);
    registration_closed_ = true;
    return StockServerOperationCatalog(std::move(registrations_));
}
} // namespace tmux_cxx::detail
