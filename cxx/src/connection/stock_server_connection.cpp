#include <tmux_cxx/connection/stock_server_connection.hpp>

#include "connection/stock_server_operation_catalog.hpp"
#include "platform/posix/owned_fd.hpp"
#include "protocol/tmux/stock_command_exchange.hpp"

#include <tmux_cxx/protocol/tmux/compatibility_policy.hpp>

#include <algorithm>
#include <array>
#include <cctype>
#include <cerrno>
#include <chrono>
#include <cstring>
#include <fcntl.h>
#include <poll.h>
#include <stdexcept>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

namespace tmux_cxx {
namespace {
using Clock = std::chrono::steady_clock; ///< Monotonic stock command deadlines.
using protocol::tmux::StockClientDescriptor;
using protocol::tmux::StockClientFrame;
using protocol::tmux::StockClientIdentification;
using protocol::tmux::StockCommandExchange;
using protocol::tmux::TmuxProtocolProfile;
constexpr auto stock_command_timeout =
    std::chrono::milliseconds(900); ///< Whole stock command exchange bound.

/** @brief Resolve only the two built-in profiles whose catalogs have stable storage. */
const TmuxProtocolProfile *declared_profile(const TmuxProtocolProfile &profile) {
    for (const auto *candidate :
         {&protocol::tmux::legacy_profile, &protocol::tmux::modern_profile}) {
        if (profile.name == candidate->name && profile.layout == candidate->layout &&
            profile.protocol_version == candidate->protocol_version &&
            profile.descriptor_marker == candidate->descriptor_marker) {
            return candidate;
        }
    }
    return nullptr;
}

/** @brief Wait for one socket condition until the command-wide monotonic deadline. */
Result<void> await_socket(int descriptor, short events, Clock::time_point deadline) {
    for (;;) {
        const auto remaining_ms =
            std::chrono::ceil<std::chrono::milliseconds>(deadline - Clock::now()).count();
        if (remaining_ms <= 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::timeout, "stock server command timed out"});
        }
        pollfd poll_request{descriptor, events, 0};
        const auto poll_result = ::poll(&poll_request, 1, static_cast<int>(remaining_ms));
        if (poll_result > 0 && (poll_request.revents & events) != 0) {
            return {};
        }
        if (poll_result > 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::closed, "stock server connection closed"});
        }
        if (poll_result == 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::timeout, "stock server command timed out"});
        }
        if (errno != EINTR) {
            return std::unexpected(TmuxError{TmuxErrorCode::io, std::string("stock server poll: ") +
                                                                    std::strerror(errno)});
        }
    }
}

/** @brief Connect one nonblocking Unix socket without starting or replacing a stock server. */
Result<posix::OwnedFd> connect_stock_server(std::string_view socket_path,
                                            Clock::time_point deadline) {
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    if (socket_path.empty() || socket_path.size() >= sizeof(address.sun_path)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid stock server socket path"});
    }
    std::memcpy(address.sun_path, socket_path.data(), socket_path.size());
    posix::OwnedFd socket_fd(::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0));
    if (socket_fd.get() < 0) {
        return std::unexpected(TmuxError{TmuxErrorCode::io, std::string("stock server socket: ") +
                                                                std::strerror(errno)});
    }
    if (::connect(socket_fd.get(), reinterpret_cast<const sockaddr *>(&address), sizeof(address)) ==
        0) {
        return socket_fd;
    }
    if (errno != EINPROGRESS) {
        return std::unexpected(TmuxError{TmuxErrorCode::io, std::string("stock server connect: ") +
                                                                std::strerror(errno)});
    }
    if (auto ready = await_socket(socket_fd.get(), POLLOUT, deadline); !ready) {
        return std::unexpected(ready.error());
    }
    int socket_error = 0;
    socklen_t error_size = sizeof(socket_error);
    if (::getsockopt(socket_fd.get(), SOL_SOCKET, SO_ERROR, &socket_error, &error_size) < 0 ||
        socket_error != 0) {
        const auto error_code = socket_error == 0 ? errno : socket_error;
        return std::unexpected(TmuxError{TmuxErrorCode::io, std::string("stock server connect: ") +
                                                                std::strerror(error_code)});
    }
    return socket_fd;
}

/** @brief Deliver one complete frame and transfer its descriptor exactly once. */
Result<void> send_client_frame(int socket_fd, const StockClientFrame &frame, int passed_descriptor,
                               Clock::time_point deadline) {
    std::size_t sent_bytes = 0;
    bool descriptor_sent = frame.descriptor == StockClientDescriptor::none;
    while (sent_bytes < frame.bytes.size()) {
        if (auto ready = await_socket(socket_fd, POLLOUT, deadline); !ready) {
            return ready;
        }
        ssize_t written_bytes;
        if (!descriptor_sent) {
            iovec payload_vector{const_cast<std::uint8_t *>(frame.bytes.data() + sent_bytes),
                                 frame.bytes.size() - sent_bytes};
            alignas(cmsghdr) std::array<unsigned char, CMSG_SPACE(sizeof(int))> control{};
            msghdr message{};
            message.msg_iov = &payload_vector;
            message.msg_iovlen = 1;
            message.msg_control = control.data();
            message.msg_controllen = control.size();
            auto *header = CMSG_FIRSTHDR(&message);
            header->cmsg_level = SOL_SOCKET;
            header->cmsg_type = SCM_RIGHTS;
            header->cmsg_len = CMSG_LEN(sizeof(int));
            std::memcpy(CMSG_DATA(header), &passed_descriptor, sizeof(passed_descriptor));
            written_bytes = ::sendmsg(socket_fd, &message, MSG_NOSIGNAL);
        } else {
            written_bytes = ::send(socket_fd, frame.bytes.data() + sent_bytes,
                                   frame.bytes.size() - sent_bytes, MSG_NOSIGNAL);
        }
        if (written_bytes > 0) {
            sent_bytes += static_cast<std::size_t>(written_bytes);
            descriptor_sent = true;
            continue;
        }
        if (written_bytes < 0 && (errno == EINTR || errno == EAGAIN)) {
            continue;
        }
        return std::unexpected(
            TmuxError{TmuxErrorCode::io, std::string("stock server write: ") +
                                             std::strerror(written_bytes == 0 ? EPIPE : errno)});
    }
    return {};
}

/** @brief Open one local null descriptor for command-mode stock identification. */
Result<posix::OwnedFd> open_null_descriptor(int flags) {
    posix::OwnedFd descriptor(::open("/dev/null", flags | O_CLOEXEC));
    if (descriptor.get() < 0) {
        return std::unexpected(TmuxError{
            TmuxErrorCode::io, std::string("open command descriptor: ") + std::strerror(errno)});
    }
    return descriptor;
}

/** @brief Execute one stock command over a new role-specific client connection. */
Result<StockCommandResult> exchange_stock_command(std::string_view socket_path,
                                                  const TmuxProtocolProfile &profile,
                                                  std::string_view working_directory,
                                                  std::span<const std::string> arguments) {
    const auto deadline = Clock::now() + stock_command_timeout;
    auto socket_fd = connect_stock_server(socket_path, deadline);
    if (!socket_fd) {
        return std::unexpected(socket_fd.error());
    }
    auto standard_input = open_null_descriptor(O_RDONLY);
    if (!standard_input) {
        return std::unexpected(standard_input.error());
    }
    auto standard_output = open_null_descriptor(O_WRONLY);
    if (!standard_output) {
        return std::unexpected(standard_output.error());
    }

    StockCommandExchange command_exchange(profile);
    StockClientIdentification identification;
    identification.terminal_type = "dumb";
    identification.working_directory = working_directory;
    identification.process_id = static_cast<std::uint32_t>(::getpid());
    identification.environment = {"TERM=dumb"};
    auto outbound_frames = command_exchange.begin_command(identification, arguments);
    if (!outbound_frames) {
        return std::unexpected(outbound_frames.error());
    }
    for (const auto &frame : *outbound_frames) {
        int descriptor = -1;
        if (frame.descriptor == StockClientDescriptor::standard_input) {
            descriptor = standard_input->get();
        } else if (frame.descriptor == StockClientDescriptor::standard_output) {
            descriptor = standard_output->get();
        }
        if (auto sent = send_client_frame(socket_fd->get(), frame, descriptor, deadline); !sent) {
            return std::unexpected(sent.error());
        }
    }

    std::array<std::uint8_t, 8192> receive_chunk{};
    for (;;) {
        if (auto ready = await_socket(socket_fd->get(), POLLIN, deadline); !ready) {
            return std::unexpected(ready.error());
        }
        const auto received_bytes =
            ::recv(socket_fd->get(), receive_chunk.data(), receive_chunk.size(), 0);
        if (received_bytes == 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::closed, "stock server closed before command completion"});
        }
        if (received_bytes < 0 && (errno == EINTR || errno == EAGAIN)) {
            continue;
        }
        if (received_bytes < 0) {
            return std::unexpected(TmuxError{TmuxErrorCode::io, std::string("stock server read: ") +
                                                                    std::strerror(errno)});
        }
        auto traffic_outcome = command_exchange.receive_server_traffic(
            std::span(receive_chunk).first(static_cast<std::size_t>(received_bytes)));
        if (!traffic_outcome) {
            return std::unexpected(traffic_outcome.error());
        }
        for (const auto &frame : traffic_outcome->client_replies) {
            if (auto sent = send_client_frame(socket_fd->get(), frame, -1, deadline); !sent) {
                return std::unexpected(sent.error());
            }
        }
        if (traffic_outcome->completed_command) {
            return std::move(*traffic_outcome->completed_command);
        }
    }
}

/** @brief Convert a bounded server-reported version line into release evidence. */
Result<std::string> read_server_release(const StockCommandResult &command_result) {
    if (command_result.exit_status != 0 || !command_result.standard_error.empty() ||
        command_result.standard_output.size() > 64) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "stock server version query failed"});
    }
    std::string release(command_result.standard_output.begin(),
                        command_result.standard_output.end());
    while (!release.empty() && (release.back() == '\n' || release.back() == '\r')) {
        release.pop_back();
    }
    const bool valid =
        !release.empty() && std::ranges::all_of(release,
                                                /** @brief Admit bounded tmux release characters. */
                                                [](unsigned char character) {
                                                    return std::isalnum(character) != 0 ||
                                                           character == '.' || character == '-' ||
                                                           character == '_';
                                                });
    if (!valid) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock server returned an invalid release"});
    }
    return release;
}
} // namespace

/** @brief Retain caller-selected endpoint policy and verified release evidence. */
struct StockServerConnection::StockServerEndpoint {
    std::string socket_path;            ///< Stock server socket selected by the caller.
    const TmuxProtocolProfile *profile; ///< Canonical static profile declaration.
    std::string working_directory;      ///< Explicit command identification directory.
    detail::StockServerOperationCatalog
        operation_catalog; ///< Frozen entity-owned reverse-adapter declarations.
    std::optional<protocol::tmux::StockConnectionCompatibility>
        compatibility_report; ///< Server-reported release and effective support.
    bool closed = false;      ///< No further connection attempts are admitted.
};

StockServerConnection::StockServerConnection(std::string socket_path,
                                             const protocol::tmux::TmuxProtocolProfile &profile,
                                             std::string working_directory) {
    const auto *selected_profile = declared_profile(profile);
    if (selected_profile == nullptr || working_directory.empty() ||
        working_directory.find('\0') != std::string::npos) {
        throw std::invalid_argument("undeclared stock profile or working directory");
    }
    auto operation_catalog = detail::build_stock_server_operation_catalog();
    if (!operation_catalog) {
        throw std::logic_error(operation_catalog.error().message);
    }
    endpoint_ = std::make_unique<StockServerEndpoint>(
        std::move(socket_path), selected_profile, std::move(working_directory),
        std::move(*operation_catalog), std::nullopt, false);
}

StockServerConnection::~StockServerConnection() = default;
StockServerConnection::StockServerConnection(StockServerConnection &&) noexcept = default;
StockServerConnection &
StockServerConnection::operator=(StockServerConnection &&) noexcept = default;

Result<protocol::tmux::StockConnectionCompatibility>
StockServerConnection::connection_compatibility() {
    if (!endpoint_ || endpoint_->closed) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::closed, "stock server connection is closed"});
    }
    if (!endpoint_->compatibility_report) {
        const std::vector<std::string> version_query{"display-message", "-p", "#{version}"};
        auto version_result = exchange_stock_command(endpoint_->socket_path, *endpoint_->profile,
                                                     endpoint_->working_directory, version_query);
        if (!version_result) {
            return std::unexpected(version_result.error());
        }
        auto release = read_server_release(*version_result);
        if (!release) {
            return std::unexpected(release.error());
        }
        endpoint_->compatibility_report =
            protocol::tmux::TmuxCompatibilityPolicy{}.assess_stock_server(
                *endpoint_->profile, *release, protocol::tmux::ConnectionMode::command);
        if (endpoint_->compatibility_report->support !=
            protocol::tmux::CapabilitySupport::unsupported) {
            endpoint_->compatibility_report->commands =
                endpoint_->operation_catalog.commands(endpoint_->compatibility_report->mode);
        }
    }
    return *endpoint_->compatibility_report;
}

Result<StockCommandResult>
StockServerConnection::exchange_operation_command(std::span<const std::string> arguments) {
    if (!endpoint_ || endpoint_->closed) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::closed, "stock server connection is closed"});
    }
    if (arguments.empty()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "explicit stock command required"});
    }
    return exchange_stock_command(endpoint_->socket_path, *endpoint_->profile,
                                  endpoint_->working_directory, arguments);
}

void StockServerConnection::close() {
    if (endpoint_) {
        endpoint_->closed = true;
    }
}
} // namespace tmux_cxx
