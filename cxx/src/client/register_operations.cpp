#include "register_operations.hpp"

#include <tmux_cxx/client/operations/attach_client.hpp>
#include <tmux_cxx/client/operations/detach_client.hpp>
#include <tmux_cxx/client/operations/list_clients.hpp>
#include <tmux_cxx/client/operations/read_stock_client_compatibility.hpp>
#include <tmux_cxx/client/operations/resize_client.hpp>

namespace tmux_cxx {
Result<void> register_client_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<ListClients>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<AttachClient>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<DetachClient>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ResizeClient>(); !registration) {
        return registration;
    }
    return registry.add<ReadStockClientCompatibility>();
}
} // namespace tmux_cxx
