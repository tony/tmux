#include "connection/native_server_channel.hpp"
#include "connection/native_request_transport.hpp"
#include "protocol/native/reply_envelope.hpp"

#include <algorithm>
#include <utility>

namespace tmux_cxx {
std::string NativeServerChannel::observed_server_instance() {
    std::lock_guard lock(server_identity_mutex);
    return server_instance;
}
TmuxError NativeServerChannel::operation_error(protocol::native::OperationDescription operation,
                                               TmuxErrorCode code, std::string message) {
    std::lock_guard lock(server_identity_mutex);
    return {code, std::move(message), std::string(operation.name), server_instance};
}
Result<protocol::native::Bytes>
NativeServerChannel::exchange_operation(protocol::native::OperationDescription operation,
                                        protocol::native::NativePayloadWriter arguments,
                                        protocol::native::Clock::time_point deadline,
                                        std::stop_token stop_token) {
    if (stop_token.stop_requested()) {
        return std::unexpected(
            operation_error(operation, TmuxErrorCode::cancelled, "native request was cancelled"));
    }
    if (request_admission_closed.load()) {
        return std::unexpected(
            operation_error(operation, TmuxErrorCode::closed, "server connection is closed"));
    }
    protocol::native::NativePayloadWriter request;
    {
        std::lock_guard lock(server_identity_mutex);
        request.string(server_instance);
    }
    request.bytes.insert(request.bytes.end(), arguments.bytes.begin(), arguments.bytes.end());
    if (request.bytes.size() > protocol::native::max_payload) {
        return std::unexpected(operation_error(operation, TmuxErrorCode::invalid_argument,
                                               "native request exceeds frame capacity"));
    }
    auto reply = request_transport.request(operation.id, request.bytes, deadline, stop_token);
    if (!reply) {
        return std::unexpected(
            operation_error(operation, reply.error().code, reply.error().message));
    }
    auto envelope = protocol::native::decode_reply_envelope(*reply);
    if (!envelope) {
        return std::unexpected(
            operation_error(operation, envelope.error().code, envelope.error().message));
    }
    {
        std::lock_guard lock(server_identity_mutex);
        if (!server_instance.empty() && server_instance != envelope->server_instance) {
            return std::unexpected(TmuxError{TmuxErrorCode::wrong_server, "server instance changed",
                                             std::string(operation.name),
                                             envelope->server_instance});
        }
        server_instance = envelope->server_instance;
    }
    if (!envelope->operation_result) {
        auto failure = std::move(envelope->operation_result.error());
        failure.operation = operation.name;
        return std::unexpected(std::move(failure));
    }
    return std::move(*envelope->operation_result);
}

Result<void> NativeServerChannel::negotiate(protocol::native::Clock::time_point deadline,
                                            std::stop_token stop_token) {
    std::unique_lock lock(negotiation_mutex);
    while (negotiation_in_progress) {
        const auto ready = negotiation_finished.wait_until(
            lock, stop_token, deadline,
            /** @brief Wake when the current handshake attempt releases negotiation ownership. */
            [this] { return !negotiation_in_progress; });
        if (!ready) {
            const auto code =
                stop_token.stop_requested() ? TmuxErrorCode::cancelled : TmuxErrorCode::timeout;
            const auto message = stop_token.stop_requested()
                                     ? "native negotiation was cancelled"
                                     : "native negotiation deadline expired";
            return std::unexpected(
                operation_error(protocol::native::handshake_description, code, message));
        }
    }
    if (negotiated_handshake) {
        return {};
    }
    if (stop_token.stop_requested()) {
        return std::unexpected(operation_error(protocol::native::handshake_description,
                                               TmuxErrorCode::cancelled,
                                               "native negotiation was cancelled"));
    }
    negotiation_in_progress = true;
    lock.unlock();
    protocol::native::NativePayloadWriter arguments;
    arguments.u16(protocol::native::protocol_version);
    auto reply = exchange_operation(protocol::native::handshake_description, std::move(arguments),
                                    deadline, stop_token);
    Result<protocol::native::NativeHandshake> handshake = std::unexpected(
        reply ? TmuxError{TmuxErrorCode::protocol, "native handshake was not decoded"}
              : reply.error());
    if (reply) {
        handshake = protocol::native::decode_handshake(*reply);
    }
    lock.lock();
    if (handshake) {
        negotiated_handshake = std::move(*handshake);
    }
    negotiation_in_progress = false;
    lock.unlock();
    negotiation_finished.notify_all();
    if (!handshake) {
        return std::unexpected(operation_error(protocol::native::handshake_description,
                                               handshake.error().code, handshake.error().message));
    }
    return {};
}
Result<protocol::native::Bytes>
NativeServerChannel::invoke_operation(protocol::native::OperationDescription operation,
                                      protocol::native::NativePayloadWriter arguments,
                                      std::stop_token stop_token) {
    const auto deadline = protocol::native::Clock::now() + std::chrono::milliseconds(900);
    if (stop_token.stop_requested()) {
        return std::unexpected(
            operation_error(operation, TmuxErrorCode::cancelled, "native request was cancelled"));
    }
    if (request_admission_closed.load()) {
        return std::unexpected(
            operation_error(operation, TmuxErrorCode::closed, "server connection is closed"));
    }
    if (auto negotiation = negotiate(deadline, stop_token); !negotiation) {
        return std::unexpected(
            operation_error(operation, negotiation.error().code, negotiation.error().message));
    }
    auto declared = std::ranges::lower_bound(
        negotiated_handshake->operations, operation.id.value, {},
        /** @brief Resolve the requested identity in the negotiated native catalog. */
        [](const protocol::native::AdvertisedOperation &advertised_operation) {
            return advertised_operation.id.value;
        });
    if (declared == negotiated_handshake->operations.end() || declared->id != operation.id ||
        (operation.required_capabilities & ~negotiated_handshake->capabilities) != 0) {
        return std::unexpected(
            operation_error(operation, TmuxErrorCode::unsupported,
                            "operation is unavailable in the negotiated native catalog"));
    }
    if (declared->name != operation.name || declared->effect != operation.effect ||
        declared->required_capabilities != operation.required_capabilities) {
        return std::unexpected(operation_error(operation, TmuxErrorCode::protocol,
                                               "native operation declaration mismatch"));
    }
    {
        std::lock_guard lock(server_identity_mutex);
        if (arguments.bytes.size() + server_instance.size() + 4 >
            negotiated_handshake->max_payload) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "request exceeds the negotiated native traffic bound",
                                             std::string(operation.name), server_instance});
        }
    }
    return exchange_operation(operation, std::move(arguments), deadline, stop_token);
}
} // namespace tmux_cxx
