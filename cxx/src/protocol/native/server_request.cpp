#include "protocol/native/server_request.hpp"
#include "protocol/native/handshake.hpp"
#include <tmux_cxx/server/server.hpp>

#include <stdexcept>

namespace tmux_cxx::protocol::native {
Result<OperationReply> invoke_request(const Frame &request, Server &server,
                                      const OperationCatalog &operation_catalog) {
    try {
        NativePayloadReader reader(request.payload);
        const auto requested_server_instance = reader.string();
        if (!requested_server_instance.empty() &&
            requested_server_instance != server.server_instance()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::wrong_server, "server instance changed"});
        }
        if (request.operation == handshake_description.id) {
            const auto requested_protocol_version = reader.u16();
            reader.require_end();
            if (requested_protocol_version != protocol_version) {
                return std::unexpected(
                    TmuxError{TmuxErrorCode::protocol, "no mutually supported native version"});
            }
            const auto registered_operations = operation_catalog.descriptions();
            return encode_handshake(registered_operations);
        }
        if (requested_server_instance.empty()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::protocol, "native handshake required before operations"});
        }
        return operation_catalog.invoke(request.operation, server, reader.remaining());
    } catch (const std::invalid_argument &error) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()});
    }
}
} // namespace tmux_cxx::protocol::native
