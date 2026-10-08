#include "server/api_catalogs.hpp"

#include "client/register_commands.hpp"
#include "client/register_key_bindings.hpp"
#include "client/register_operations.hpp"
#include "commands/command_registry.hpp"
#include "key_table/key_binding_registry.hpp"
#include "pane/register_commands.hpp"
#include "pane/register_key_bindings.hpp"
#include "pane/register_operations.hpp"
#include "paste_buffer/register_commands.hpp"
#include "paste_buffer/register_operations.hpp"
#include "protocol/native/operation_registry.hpp"
#include "session/register_commands.hpp"
#include "session/register_operations.hpp"
#include "window/register_operations.hpp"
#include "window_link/register_operations.hpp"

#include <tmux_cxx/protocol/native/api_operations.hpp>

#include <utility>

namespace tmux_cxx {
Result<ServerApiCatalogs> build_server_api_catalogs() {
    protocol::native::OperationRegistry native_registry;
    if (auto registration = register_session_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_pane_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_window_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_window_link_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_client_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_paste_buffer_operations(native_registry); !registration) {
        return std::unexpected(registration.error());
    }
    auto native_operations = std::move(native_registry).freeze(protocol::native::api_operations());
    if (!native_operations) {
        return std::unexpected(native_operations.error());
    }
    commands::CommandRegistry command_registry;
    if (auto registration = register_session_commands(command_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_pane_commands(command_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_client_commands(command_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_paste_buffer_commands(command_registry); !registration) {
        return std::unexpected(registration.error());
    }
    auto command_catalog = std::move(command_registry).freeze(*native_operations);
    if (!command_catalog) {
        return std::unexpected(command_catalog.error());
    }
    key_table::KeyBindingRegistry key_binding_registry;
    if (auto registration = register_pane_key_bindings(key_binding_registry); !registration) {
        return std::unexpected(registration.error());
    }
    if (auto registration = register_client_key_bindings(key_binding_registry); !registration) {
        return std::unexpected(registration.error());
    }
    auto key_tables = std::move(key_binding_registry).freeze(*native_operations);
    if (!key_tables) {
        return std::unexpected(key_tables.error());
    }
    return ServerApiCatalogs{std::move(*native_operations), std::move(*command_catalog),
                             std::move(*key_tables)};
}
} // namespace tmux_cxx
