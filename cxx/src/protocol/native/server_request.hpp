#pragma once

#include "protocol/native/operation_catalog.hpp"

namespace tmux_cxx::protocol::native {
/** @brief Negotiate native traffic or validate server identity before registered entity execution.
 */
Result<OperationReply> invoke_request(const Frame &request, Server &server,
                                      const OperationCatalog &operation_catalog);
} // namespace tmux_cxx::protocol::native
