#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>

#include <span>
#include <stop_token>
#include <string>

namespace tmux_cxx {
/** @brief Send isolated native request exchanges to one server endpoint. */
class NativeRequestTransport {
  public:
    /** @brief Derive the private native endpoint from a tmux server socket path. */
    explicit NativeRequestTransport(std::string server_socket_path);
    /** @brief Send one framed operation and return its matched reply before cancellation or the
     * deadline. */
    Result<protocol::native::Bytes> request(protocol::native::OperationId operation,
                                            std::span<const std::uint8_t> payload,
                                            protocol::native::Clock::time_point deadline,
                                            std::stop_token stop_token = {}) const;

  private:
    std::string native_socket_path_; ///< Private native endpoint used for fresh request sockets.
};
} // namespace tmux_cxx
