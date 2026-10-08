#pragma once

#include "commands/command_catalog.hpp"
#include "key_table/key_table_catalog.hpp"
#include "protocol/native/operation_catalog.hpp"

namespace tmux_cxx {
/** @brief Own immutable operation, command, and key-table catalogs used by one server daemon. */
struct ServerApiCatalogs {
    protocol::native::OperationCatalog native_operations; ///< Typed native API lookup.
    commands::CommandCatalog stock_commands;              ///< Typed stock command lookup.
    key_table::KeyTableCatalog key_tables;                ///< Typed interactive binding lookup.
};

/** @brief Register every entity API, validate cross-catalog references, and freeze its catalogs.
 */
Result<ServerApiCatalogs> build_server_api_catalogs();
} // namespace tmux_cxx
