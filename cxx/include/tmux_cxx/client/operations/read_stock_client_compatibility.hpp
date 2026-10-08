#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>

namespace tmux_cxx {
/** @brief Describe the typed client.read_stock_compatibility operation and its native identity. */
struct ReadStockClientCompatibility {
    /** @brief Identify the live stock client whose selected protocol policy should be copied. */
    struct Request {
        ClientId client; ///< Connection-lifetime client resolved when observation executes.
    };
    /** @brief Copied profile, support, evidence, quirks, and limitations for this connection. */
    using Response = protocol::tmux::StockConnectionCompatibility;
    /** @brief Immutable identity and effect metadata borrowed by native registration. */
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{17}, "client.read_stock_compatibility",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(
            protocol::native::Capability::stock_client_compatibility)};
    /** @brief Decode one client identity without observing server state. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::stock_client_compatibility */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode one copied compatibility report without live adapter references. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
