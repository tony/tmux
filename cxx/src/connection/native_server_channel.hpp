#pragma once

#include <tmux_cxx/connection/server_connection.hpp>

#include "connection/native_request_transport.hpp"
#include "protocol/native/framing.hpp"
#include "protocol/native/handshake.hpp"

#include <atomic>
#include <condition_variable>
#include <mutex>
#include <stop_token>

namespace tmux_cxx {
/** @brief Own admission state and the observed server identity across native request connections.
 */
class NativeServerChannel {
  public:
    /** @brief Retain the native endpoint without starting a daemon or opening a connection. */
    explicit NativeServerChannel(std::string socket_path)
        : request_transport(std::move(socket_path)) {}
    /** @brief Bind a request to the observed server and retain operation context on failures. */
    Result<protocol::native::Bytes>
    invoke_operation(protocol::native::OperationDescription operation,
                     protocol::native::NativePayloadWriter arguments,
                     std::stop_token stop_token = {});
    /** @brief Attach the requested operation and pinned server identity to a local failure. */
    TmuxError operation_error(protocol::native::OperationDescription operation, TmuxErrorCode code,
                              std::string message);
    /** @brief Copy the server identity pinned by the latest accepted native reply. */
    std::string observed_server_instance();
    /** @brief Negotiate once within the complete request deadline; retain the selected server
     * catalog. */
    Result<void> negotiate(protocol::native::Clock::time_point deadline,
                           std::stop_token stop_token);
    /** @brief Exchange an identity-bound request before cancellation or its established deadline.
     */
    Result<protocol::native::Bytes>
    exchange_operation(protocol::native::OperationDescription operation,
                       protocol::native::NativePayloadWriter arguments,
                       protocol::native::Clock::time_point deadline, std::stop_token stop_token);

    NativeRequestTransport request_transport; ///< Fresh sockets for bounded native requests.
    std::atomic<bool> request_admission_closed =
        false;                        ///< Closed admission preserves persistent server entities.
    std::mutex server_identity_mutex; ///< Protect identity across binding worker threads.
    std::string server_instance;      ///< Identity pinned after an accepted native reply.
    std::mutex negotiation_mutex;     ///< Protect the one-time native catalog negotiation.
    std::condition_variable_any
        negotiation_finished;             ///< Wake waiters after negotiation success or failure.
    bool negotiation_in_progress = false; ///< One caller currently owns handshake traffic.
    std::optional<protocol::native::NativeHandshake>
        negotiated_handshake; ///< Immutable negotiated operation catalog.
};
} // namespace tmux_cxx
