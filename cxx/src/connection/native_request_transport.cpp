#include "connection/native_request_transport.hpp"

#include "platform/posix/errno_error.hpp"
#include "platform/posix/owned_fd.hpp"
#include "protocol/native/framing.hpp"

#include <array>
#include <cerrno>
#include <cstring>
#include <poll.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Describe a caller-requested stop without conflating it with timeout or peer closure. */
TmuxError cancellation_error() {
    return {TmuxErrorCode::cancelled, "native request was cancelled"};
}

/** @brief Await socket readiness until cancellation or the request's absolute deadline. */
Result<void> await_socket(int socket_fd, short events, protocol::native::Clock::time_point deadline,
                          std::stop_token stop_token) {
    for (;;) {
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        auto remaining_time = std::chrono::duration_cast<std::chrono::milliseconds>(
            deadline - protocol::native::Clock::now());
        if (remaining_time.count() <= 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::timeout, "native request deadline expired"});
        }
        pollfd poll_request{socket_fd, events, 0};
        auto poll_result = ::poll(&poll_request, 1, static_cast<int>(remaining_time.count()));
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        if (poll_result < 0 && errno == EINTR) {
            continue;
        }
        if (poll_result < 0) {
            return std::unexpected(posix::errno_error("poll native request socket"));
        }
        if (poll_result > 0 && (poll_request.revents & events) != 0) {
            return {};
        }
        if (poll_result > 0) {
            return std::unexpected(TmuxError{TmuxErrorCode::io, "native server disconnected"});
        }
    }
}

/** @brief Open one nonblocking native request socket before installing cancellation. */
Result<posix::OwnedFd> open_socket() {
    posix::OwnedFd socket_fd(::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0));
    if (socket_fd.get() < 0) {
        return std::unexpected(posix::errno_error("open native request socket"));
    }
    return socket_fd;
}

/** @brief Connect an opened socket to the private native endpoint before cancellation or deadline.
 */
Result<void> connect_socket(int socket_fd, const std::string &native_socket_path,
                            protocol::native::Clock::time_point deadline,
                            std::stop_token stop_token) {
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    if (native_socket_path.size() >= sizeof(address.sun_path)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "native socket path is too long"});
    }
    std::memcpy(address.sun_path, native_socket_path.c_str(), native_socket_path.size() + 1);
    if (stop_token.stop_requested()) {
        return std::unexpected(cancellation_error());
    }
    if (::connect(socket_fd, reinterpret_cast<const sockaddr *>(&address), sizeof(address)) == 0) {
        return {};
    }
    if (stop_token.stop_requested()) {
        return std::unexpected(cancellation_error());
    }
    if (errno != EINPROGRESS) {
        return std::unexpected(posix::errno_error("connect native request socket"));
    }
    auto connection_ready = await_socket(socket_fd, POLLOUT, deadline, stop_token);
    if (!connection_ready) {
        return std::unexpected(connection_ready.error());
    }
    int socket_error = 0;
    socklen_t error_size = sizeof(socket_error);
    if (::getsockopt(socket_fd, SOL_SOCKET, SO_ERROR, &socket_error, &error_size) < 0) {
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        return std::unexpected(posix::errno_error("inspect native request socket"));
    }
    if (socket_error != 0) {
        return std::unexpected(TmuxError{TmuxErrorCode::io, std::strerror(socket_error)});
    }
    return {};
}

/** @brief Send one complete native frame without retrying the logical request. */
Result<void> send_frame(int socket_fd, std::span<const std::uint8_t> request_frame,
                        protocol::native::Clock::time_point deadline, std::stop_token stop_token) {
    std::size_t sent_bytes = 0;
    while (sent_bytes < request_frame.size()) {
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        auto written_bytes = ::send(socket_fd, request_frame.data() + sent_bytes,
                                    request_frame.size() - sent_bytes, MSG_NOSIGNAL);
        if (written_bytes > 0) {
            sent_bytes += static_cast<std::size_t>(written_bytes);
            continue;
        }
        if (written_bytes < 0 && errno == EINTR) {
            continue;
        }
        if (written_bytes < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            auto write_ready = await_socket(socket_fd, POLLOUT, deadline, stop_token);
            if (!write_ready) {
                return std::unexpected(write_ready.error());
            }
            continue;
        }
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        if (written_bytes == 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::io, "native request socket closed during send"});
        }
        return std::unexpected(posix::errno_error("send native request frame"));
    }
    return {};
}

/** @brief Receive exactly one complete bounded native reply frame. */
Result<protocol::native::Bytes> receive_frame(int socket_fd,
                                              protocol::native::Clock::time_point deadline,
                                              std::stop_token stop_token) {
    protocol::native::Bytes reply_frame;
    std::array<std::uint8_t, 8192> receive_chunk{};
    std::size_t expected_frame_size = protocol::native::header_size;
    while (reply_frame.size() < expected_frame_size) {
        auto read_ready = await_socket(socket_fd, POLLIN, deadline, stop_token);
        if (!read_ready) {
            return std::unexpected(read_ready.error());
        }
        auto received_bytes = ::recv(socket_fd, receive_chunk.data(), receive_chunk.size(), 0);
        if (received_bytes < 0 && (errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK)) {
            continue;
        }
        if (stop_token.stop_requested()) {
            return std::unexpected(cancellation_error());
        }
        if (received_bytes < 0) {
            return std::unexpected(posix::errno_error("receive native reply frame"));
        }
        if (received_bytes == 0) {
            return std::unexpected(TmuxError{TmuxErrorCode::io, "native reply was truncated"});
        }
        reply_frame.insert(reply_frame.end(), receive_chunk.begin(),
                           receive_chunk.begin() + received_bytes);
        if (reply_frame.size() >= protocol::native::header_size) {
            auto reply_frame_size = protocol::native::frame_size(reply_frame);
            if (!reply_frame_size) {
                return std::unexpected(reply_frame_size.error());
            }
            expected_frame_size = *reply_frame_size;
        }
    }
    return reply_frame;
}
} // namespace

NativeRequestTransport::NativeRequestTransport(std::string server_socket_path)
    : native_socket_path_(std::move(server_socket_path) + ".native") {}

Result<protocol::native::Bytes> NativeRequestTransport::request(
    protocol::native::OperationId operation, std::span<const std::uint8_t> payload,
    protocol::native::Clock::time_point deadline, std::stop_token stop_token) const {
    if (stop_token.stop_requested()) {
        return std::unexpected(cancellation_error());
    }
    if (protocol::native::Clock::now() >= deadline) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::timeout, "native request deadline expired"});
    }
    auto socket_fd = open_socket();
    if (!socket_fd) {
        return std::unexpected(socket_fd.error());
    }
    /** @brief Interrupt an admitted socket operation when its request is cancelled. */
    auto interrupt_socket = [descriptor = socket_fd->get()] {
        (void)::shutdown(descriptor, SHUT_RDWR);
    };
    std::stop_callback cancel_socket(stop_token, std::move(interrupt_socket));
    if (auto connected =
            connect_socket(socket_fd->get(), native_socket_path_, deadline, stop_token);
        !connected) {
        return std::unexpected(connected.error());
    }
    auto request_frame = protocol::native::frame(operation, payload);
    if (auto sent = send_frame(socket_fd->get(), request_frame, deadline, stop_token); !sent) {
        return std::unexpected(sent.error());
    }
    auto reply_frame = receive_frame(socket_fd->get(), deadline, stop_token);
    if (!reply_frame) {
        return std::unexpected(reply_frame.error());
    }
    auto decoded_reply = protocol::native::decode(*reply_frame);
    if (!decoded_reply) {
        return std::unexpected(decoded_reply.error());
    }
    if (decoded_reply->operation != operation) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "native reply operation mismatch"});
    }
    return std::move(decoded_reply->payload);
}
} // namespace tmux_cxx
