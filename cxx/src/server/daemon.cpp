#include <tmux_cxx/server/server.hpp>

#include "client/client_terminal_routing.hpp"
#include "client/client_view_refresh.hpp"
#include "client/stock_client_lifecycle.hpp"
#include "platform/posix/owned_fd.hpp"
#include "protocol/native/framing.hpp"
#include "protocol/native/reply_envelope.hpp"
#include "protocol/native/server_request.hpp"
#include "protocol/tmux/accepted_client_protocol.hpp"
#include "server/api_catalogs.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <chrono>
#include <csignal>
#include <cstring>
#include <iostream>
#include <optional>
#include <poll.h>
#include <stdexcept>
#include <sys/signalfd.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>
#include <variant>

namespace tmux_cxx {
namespace {
using Clock = std::chrono::steady_clock; ///< Monotonic request deadlines.
using protocol::native::Bytes;           ///< Shared encoded wire-byte container.
using protocol::native::OperationId;
namespace native = protocol::native;
namespace tmux_wire = protocol::tmux;
using posix::OwnedFd;

/** @brief Own a private Unix listener and remove only its original filesystem identity. */
class UnixSocketListener {
  public:
    /** @brief Bind a new endpoint and reject existing socket paths without replacing them. */
    explicit UnixSocketListener(std::string socket_path) : socket_path_(std::move(socket_path)) {
        sockaddr_un address{};
        address.sun_family = AF_UNIX;
        if (socket_path_.size() >= sizeof(address.sun_path)) {
            throw std::runtime_error("socket path is too long");
        }
        std::memcpy(address.sun_path, socket_path_.c_str(), socket_path_.size() + 1);
        listening_socket_.reset(::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0));
        if (listening_socket_.get() < 0 ||
            ::bind(listening_socket_.get(), reinterpret_cast<const sockaddr *>(&address),
                   sizeof(address)) < 0) {
            throw std::runtime_error(std::string("socket bind: ") + std::strerror(errno));
        }
        if (::lstat(socket_path_.c_str(), &socket_identity_) < 0 ||
            ::chmod(socket_path_.c_str(), 0600) < 0 || ::listen(listening_socket_.get(), 64) < 0) {
            ::unlink(socket_path_.c_str());
            throw std::runtime_error(std::string("socket listen: ") + std::strerror(errno));
        }
    }
    /** @brief Unlink the owned socket only if its device and inode still match. */
    ~UnixSocketListener() {
        struct stat current_identity{};
        if (::lstat(socket_path_.c_str(), &current_identity) == 0 &&
            current_identity.st_dev == socket_identity_.st_dev &&
            current_identity.st_ino == socket_identity_.st_ino) {
            ::unlink(socket_path_.c_str());
        }
    }
    /** @brief Borrow the listening descriptor until this listener is destroyed. */
    int descriptor() const { return listening_socket_.get(); }

  private:
    std::string socket_path_;  ///< Owned filesystem endpoint used for identity-checked cleanup.
    OwnedFd listening_socket_; ///< Owned nonblocking listener descriptor.
    struct stat socket_identity_{}; ///< Device and inode established when binding the endpoint.
};

/** @brief Retain one bounded native request and its operation-owned continuation. */
struct NativeRequestState {
    Bytes receive_buffer; ///< Incomplete native frames bounded by transport capacity.
    std::optional<native::PendingReply>
        pending_reply;               ///< Continuation awaiting readiness or its deadline.
    OperationId request_operation{}; ///< Identity retained while a native reply is pending.
};

/** @brief Name when an accepted transport connection may be disposed. */
enum class ConnectionClosure {
    keep_open,   ///< Continue receiving and sending protocol traffic.
    after_flush, ///< Close after queued output has reached the remote endpoint.
    immediate    ///< Close during the current event-loop turn.
};

/** @brief Select the protocol role assigned to a newly accepted connection. */
enum class AcceptedProtocol {
    native_api, ///< Receive one native API request and close after its reply.
    stock_tmux  ///< Serve a stock tmux client through its connection lifetime.
};

/** @brief Own one accepted transport separately from its native or stock protocol role. */
struct AcceptedConnection {
    OwnedFd socket;                  ///< Owned connected transport descriptor.
    Bytes outbound_bytes;            ///< Encoded protocol traffic awaiting nonblocking delivery.
    std::size_t delivered_bytes = 0; ///< Prefix already delivered from the outbound bytes.
    ConnectionClosure closure = ConnectionClosure::keep_open; ///< Requested disposal point.
    std::variant<NativeRequestState, tmux_wire::AcceptedClientProtocol> protocol_state;
    ///< Closed wire states with independent frame, identification, and continuation ownership.
};

/** @brief Bind borrowed control descriptors to their stable connection index for one poll turn. */
struct StockControlModeRoute {
    std::size_t connection_index; ///< Accepted connection owning the control protocol state.
    tmux_wire::AcceptedClientProtocol::ControlModeRoute route; ///< Borrowed descriptor readiness.
};

/** @brief Queue encoded traffic or close a connection that exceeds its outbound bound. */
void queue_transport_bytes(AcceptedConnection &connection, Bytes bytes) {
    if (connection.outbound_bytes.size() - connection.delivered_bytes + bytes.size() >
        protocol::native::max_payload + 4096) {
        connection.closure = ConnectionClosure::immediate;
        return;
    }
    connection.outbound_bytes.insert(connection.outbound_bytes.end(), bytes.begin(), bytes.end());
}
/** @brief Apply one accepted stock protocol action to its separately owned transport. */
void apply_stock_transport_action(
    AcceptedConnection &connection,
    Result<tmux_wire::AcceptedClientProtocol::TransportAction> transport_action) {
    if (!transport_action) {
        connection.closure = ConnectionClosure::immediate;
        return;
    }
    queue_transport_bytes(connection, std::move(transport_action->outbound_bytes));
    if (transport_action->close_after_write && connection.closure != ConnectionClosure::immediate) {
        connection.closure = ConnectionClosure::after_flush;
    }
}
/** @brief Encode a native result with server identity before closing this request connection. */
void queue_native_reply(AcceptedConnection &connection, Server &server, OperationId operation,
                        const Result<Bytes> &operation_result) {
    auto reply_envelope = native::encode_reply_envelope(server.server_instance(), operation_result);
    queue_transport_bytes(connection, native::frame(operation, reply_envelope));
    if (connection.closure != ConnectionClosure::immediate) {
        connection.closure = ConnectionClosure::after_flush;
    }
}
/** @brief Validate server identity and invoke the frozen native operation catalog. */
void invoke_native_request(AcceptedConnection &connection, Server &server,
                           const native::OperationCatalog &operation_catalog,
                           const native::Frame &request_frame) {
    auto &native_request = std::get<NativeRequestState>(connection.protocol_state);
    native_request.request_operation = request_frame.operation;
    try {
        auto operation_reply = native::invoke_request(request_frame, server, operation_catalog);
        if (!operation_reply) {
            queue_native_reply(connection, server, request_frame.operation,
                               std::unexpected(operation_reply.error()));
        } else if (auto reply_bytes = std::get_if<native::Bytes>(&*operation_reply)) {
            queue_native_reply(connection, server, request_frame.operation,
                               std::move(*reply_bytes));
        } else {
            native_request.pending_reply =
                std::move(std::get<native::PendingReply>(*operation_reply));
        }
    } catch (const std::exception &error) {
        queue_native_reply(connection, server, request_frame.operation,
                           std::unexpected(TmuxError{TmuxErrorCode::protocol, error.what()}));
    }
}

/** @brief Assemble bounded frames and retain transferred descriptors before invoking the
 * appropriate adapter. */
void receive_connection_traffic(AcceptedConnection &connection, Server &server,
                                const native::OperationCatalog &operation_catalog,
                                const commands::CommandCatalog &command_catalog,
                                tmux_wire::control::CommandSequence &control_sequence) {
    std::array<std::uint8_t, 8192> received_bytes{};
    alignas(cmsghdr) std::array<unsigned char, CMSG_SPACE(16 * sizeof(int))> descriptor_control{};
    iovec receive_vector{received_bytes.data(), received_bytes.size()};
    msghdr receive_message{};
    receive_message.msg_iov = &receive_vector;
    receive_message.msg_iovlen = 1;
    receive_message.msg_control = descriptor_control.data();
    receive_message.msg_controllen = descriptor_control.size();
    const auto received_byte_count =
        ::recvmsg(connection.socket.get(), &receive_message, MSG_CMSG_CLOEXEC);
    if (received_byte_count < 0 && (errno == EINTR || errno == EAGAIN)) {
        return;
    }
    if (received_byte_count <= 0) {
        connection.closure = ConnectionClosure::immediate;
        return;
    }
    std::vector<OwnedFd> received_descriptors;
    for (auto *control_header = CMSG_FIRSTHDR(&receive_message); control_header != nullptr;
         control_header = CMSG_NXTHDR(&receive_message, control_header)) {
        if (control_header->cmsg_level == SOL_SOCKET && control_header->cmsg_type == SCM_RIGHTS) {
            const auto descriptor_count = (control_header->cmsg_len - CMSG_LEN(0)) / sizeof(int);
            for (std::size_t descriptor_index = 0; descriptor_index < descriptor_count;
                 ++descriptor_index) {
                int transferred_descriptor =
                    -1; ///< Transferred descriptor adopted before validating ancillary traffic.
                std::memcpy(&transferred_descriptor,
                            CMSG_DATA(control_header) + descriptor_index * sizeof(int),
                            sizeof(transferred_descriptor));
                received_descriptors.emplace_back(transferred_descriptor);
            }
        }
    }
    if ((receive_message.msg_flags & (MSG_CTRUNC | MSG_TRUNC)) != 0 ||
        received_descriptors.size() > 16 ||
        (std::holds_alternative<NativeRequestState>(connection.protocol_state) &&
         !received_descriptors.empty())) {
        connection.closure = ConnectionClosure::immediate;
        return;
    }
    if (auto *client_protocol =
            std::get_if<tmux_wire::AcceptedClientProtocol>(&connection.protocol_state)) {
        auto transport_action = client_protocol->receive_client_traffic(
            std::span(received_bytes).first(static_cast<std::size_t>(received_byte_count)),
            std::move(received_descriptors), server, command_catalog, control_sequence);
        apply_stock_transport_action(connection, std::move(transport_action));
        return;
    }
    auto &native_request = std::get<NativeRequestState>(connection.protocol_state);
    auto &receive_buffer = native_request.receive_buffer;
    receive_buffer.insert(receive_buffer.end(), received_bytes.begin(),
                          received_bytes.begin() + received_byte_count);
    if (receive_buffer.size() > native::max_payload + native::header_size) {
        connection.closure = ConnectionClosure::immediate;
        return;
    }
    try {
        if (receive_buffer.size() < native::header_size) {
            return;
        }
        auto complete_frame_size = native::frame_size(receive_buffer);
        if (!complete_frame_size) {
            connection.closure = ConnectionClosure::immediate;
            return;
        }
        if (receive_buffer.size() >= *complete_frame_size) {
            auto frame = native::decode(receive_buffer);
            if (!frame) {
                connection.closure = ConnectionClosure::immediate;
                return;
            }
            invoke_native_request(connection, server, operation_catalog, *frame);
            receive_buffer.clear();
        }
    } catch (const std::exception &) {
        connection.closure = ConnectionClosure::immediate;
    }
}

/** @brief Flush queued protocol traffic and honor closure only after all bytes are sent. */
void flush_connection_output(AcceptedConnection &connection) {
    const auto sent_byte_count = ::send(
        connection.socket.get(), connection.outbound_bytes.data() + connection.delivered_bytes,
        connection.outbound_bytes.size() - connection.delivered_bytes, MSG_NOSIGNAL);
    if (sent_byte_count > 0) {
        connection.delivered_bytes += static_cast<std::size_t>(sent_byte_count);
    } else if (sent_byte_count < 0 && errno != EINTR && errno != EAGAIN) {
        connection.closure = ConnectionClosure::immediate;
    }
    if (connection.delivered_bytes == connection.outbound_bytes.size()) {
        connection.delivered_bytes = 0;
        connection.outbound_bytes.clear();
        if (connection.closure == ConnectionClosure::after_flush) {
            connection.closure = ConnectionClosure::immediate;
        }
    }
}

/** @brief Admit a same-user descriptor under the requested protocol role and connection bound. */
void accept_connection(int listening_socket, AcceptedProtocol accepted_protocol, Server &server,
                       std::vector<AcceptedConnection> &connections) {
    const int accepted_socket =
        ::accept4(listening_socket, nullptr, nullptr, SOCK_CLOEXEC | SOCK_NONBLOCK);
    if (accepted_socket < 0) {
        return;
    }
    OwnedFd accepted_socket_owner(accepted_socket);
    ucred peer_credential{};
    socklen_t credential_size = sizeof(peer_credential);
    if (connections.size() >= 128 ||
        ::getsockopt(accepted_socket, SOL_SOCKET, SO_PEERCRED, &peer_credential, &credential_size) <
            0 ||
        peer_credential.uid != ::getuid()) {
        return;
    }
    AcceptedConnection connection{};
    connection.socket = std::move(accepted_socket_owner);
    if (accepted_protocol == AcceptedProtocol::stock_tmux) {
        auto client_id = StockClientLifecycle::accept(server);
        if (!client_id) {
            return;
        }
        connection.protocol_state.emplace<tmux_wire::AcceptedClientProtocol>(*client_id);
    } else {
        connection.protocol_state.emplace<NativeRequestState>();
    }
    connections.push_back(std::move(connection));
}

/** @brief Dispose closed or expired connections and end their connection-lifetime clients. */
void dispose_closed_connections(Server &server, std::vector<AcceptedConnection> &connections,
                                Clock::time_point now) {
    for (auto connection_entry = connections.begin(); connection_entry != connections.end();) {
        const auto *client_protocol =
            std::get_if<tmux_wire::AcceptedClientProtocol>(&connection_entry->protocol_state);
        const auto protocol_deadline =
            client_protocol ? client_protocol->next_protocol_deadline() : std::nullopt;
        if (connection_entry->closure != ConnectionClosure::immediate &&
            (!protocol_deadline || now < *protocol_deadline)) {
            ++connection_entry;
            continue;
        }
        if (client_protocol) {
            StockClientLifecycle::disconnect(server, client_protocol->client_id());
        }
        connection_entry = connections.erase(connection_entry);
    }
}

/** @brief Close the accepted stock transport that owns one failed tmux client route. */
void close_stock_client_transport(std::vector<AcceptedConnection> &connections, ClientId client) {
    for (auto &connection : connections) {
        const auto *client_protocol =
            std::get_if<tmux_wire::AcceptedClientProtocol>(&connection.protocol_state);
        if (client_protocol && client_protocol->client_id() == client) {
            connection.closure = ConnectionClosure::immediate;
            return;
        }
    }
}

/** @brief Deliver stock lifecycle traffic derived from copied client attachment state. */
void advance_stock_client_protocols(Server &server, std::vector<AcceptedConnection> &connections) {
    const auto client_snapshots = server.clients();
    for (auto &connection : connections) {
        auto *client_protocol =
            std::get_if<tmux_wire::AcceptedClientProtocol>(&connection.protocol_state);
        if (!client_protocol || connection.closure == ConnectionClosure::immediate) {
            continue;
        }
        const auto client_snapshot =
            std::ranges::find(client_snapshots, client_protocol->client_id(), &ClientSnapshot::id);
        if (client_snapshot == client_snapshots.end()) {
            connection.closure = ConnectionClosure::immediate;
            continue;
        }
        apply_stock_transport_action(
            connection, client_protocol->advance_server_driven_output(server, *client_snapshot));
    }
}
} // namespace

int run_server(const std::string &stock_socket_path, int readiness_descriptor) {
    sigset_t daemon_signal_mask;
    ::sigemptyset(&daemon_signal_mask);
    for (int signal : {SIGTERM, SIGINT, SIGCHLD}) {
        ::sigaddset(&daemon_signal_mask, signal);
    }
    if (::sigprocmask(SIG_BLOCK, &daemon_signal_mask, nullptr) < 0) {
        throw std::runtime_error("cannot block daemon signals");
    }
    OwnedFd signal_descriptor(::signalfd(-1, &daemon_signal_mask, SFD_CLOEXEC | SFD_NONBLOCK));
    if (signal_descriptor.get() < 0) {
        throw std::runtime_error("cannot create daemon signal descriptor");
    }
    Server server;
    tmux_wire::control::CommandSequence control_command_sequence;
    auto api_catalogs = build_server_api_catalogs();
    if (!api_catalogs) {
        throw std::runtime_error(api_catalogs.error().operation + ": " +
                                 api_catalogs.error().message);
    }
    UnixSocketListener stock_listener(stock_socket_path);
    UnixSocketListener native_listener(stock_socket_path + ".native");
    if (readiness_descriptor >= 0) {
        OwnedFd readiness_signal(readiness_descriptor);
        if (::write(readiness_signal.get(), "R", 1) != 1) {
            throw std::runtime_error("cannot signal daemon readiness");
        }
    }
    std::vector<AcceptedConnection> connections;
    bool running = true;
    while (running) {
        server.reap_pane_programs();
        const auto now = Clock::now();
        dispose_closed_connections(server, connections, now);
        for (const auto client : ClientViewRefresh::queue_invalidated(server)) {
            close_stock_client_transport(connections, client);
        }
        advance_stock_client_protocols(server, connections);
        dispose_closed_connections(server, connections, now);
        std::vector<pollfd> poll_descriptors{{stock_listener.descriptor(), POLLIN, 0},
                                             {native_listener.descriptor(), POLLIN, 0},
                                             {signal_descriptor.get(), POLLIN, 0}};
        int poll_timeout_ms = -1;
        if (const auto program_deadline = server.next_pane_program_deadline()) {
            poll_timeout_ms = static_cast<int>(std::max<std::int64_t>(
                0, std::chrono::ceil<std::chrono::milliseconds>(*program_deadline - Clock::now())
                       .count()));
        }
        for (auto &connection : connections) {
            if (const auto *client_protocol =
                    std::get_if<tmux_wire::AcceptedClientProtocol>(&connection.protocol_state)) {
                if (const auto protocol_deadline = client_protocol->next_protocol_deadline()) {
                    const auto remaining = std::chrono::ceil<std::chrono::milliseconds>(
                                               *protocol_deadline - Clock::now())
                                               .count();
                    const int milliseconds = static_cast<int>(std::max<std::int64_t>(0, remaining));
                    poll_timeout_ms = poll_timeout_ms < 0 ? milliseconds
                                                          : std::min(poll_timeout_ms, milliseconds);
                }
            }
            short connection_events = POLLIN;
            if (!connection.outbound_bytes.empty()) {
                connection_events |= POLLOUT;
            }
            poll_descriptors.push_back({connection.socket.get(), connection_events, 0});
            if (const auto *native_request =
                    std::get_if<NativeRequestState>(&connection.protocol_state);
                native_request && native_request->pending_reply) {
                auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(
                                     native_request->pending_reply->deadline - Clock::now())
                                     .count();
                const int milliseconds = static_cast<int>(std::max<std::int64_t>(0, remaining));
                poll_timeout_ms =
                    poll_timeout_ms < 0 ? milliseconds : std::min(poll_timeout_ms, milliseconds);
            }
        }
        auto pane_pty_routes = server.pane_pty_descriptors();
        for (const auto &[pane_id, pty_descriptor] : pane_pty_routes) {
            const short pane_events = (server.pane_pty_read_blocked(pane_id) ? 0 : POLLIN) |
                                      (server.pane_pty_write_pending(pane_id) ? POLLOUT : 0);
            poll_descriptors.push_back({pty_descriptor, pane_events, 0});
        }
        const auto client_routes = ClientTerminalRouting::capture(server);
        for (const auto &client_route : client_routes) {
            poll_descriptors.push_back({client_route.input_descriptor,
                                        static_cast<short>(client_route.input_enabled ? POLLIN : 0),
                                        0});
            poll_descriptors.push_back(
                {client_route.output_descriptor,
                 static_cast<short>(client_route.output_pending ? POLLOUT : 0), 0});
        }
        std::vector<StockControlModeRoute> control_routes;
        for (std::size_t connection_index = 0; connection_index < connections.size();
             ++connection_index) {
            const auto *client_protocol = std::get_if<tmux_wire::AcceptedClientProtocol>(
                &connections[connection_index].protocol_state);
            if (!client_protocol ||
                connections[connection_index].closure == ConnectionClosure::immediate) {
                continue;
            }
            if (auto control_route = client_protocol->control_mode_route()) {
                control_routes.push_back({connection_index, *control_route});
                poll_descriptors.push_back(
                    {control_route->input_descriptor,
                     static_cast<short>(control_route->accepts_commands ? POLLIN : 0), 0});
                poll_descriptors.push_back(
                    {control_route->output_descriptor,
                     static_cast<short>(control_route->output_pending ? POLLOUT : 0), 0});
            }
        }
        for (const auto program_readiness_descriptor :
             server.pane_program_readiness_descriptors()) {
            poll_descriptors.push_back({program_readiness_descriptor, POLLIN, 0});
        }
        const auto ready_descriptor_count =
            ::poll(poll_descriptors.data(), poll_descriptors.size(), poll_timeout_ms);
        if (ready_descriptor_count < 0 && errno == EINTR) {
            continue;
        }
        if (ready_descriptor_count < 0) {
            throw std::runtime_error("daemon poll failed");
        }
        if (poll_descriptors[2].revents != 0) {
            signalfd_siginfo signal_info{};
            while (::read(signal_descriptor.get(), &signal_info, sizeof(signal_info)) ==
                   sizeof(signal_info)) {
                if (signal_info.ssi_signo == SIGTERM || signal_info.ssi_signo == SIGINT) {
                    running = false;
                } else if (signal_info.ssi_signo == SIGCHLD) {
                    server.reap_pane_programs();
                }
            }
        }
        const auto connection_count = connections.size();
        for (std::size_t i = 0; i < connection_count; ++i) {
            auto &connection = connections[i];
            const auto connection_events = poll_descriptors[i + 3].revents;
            if ((connection_events & POLLIN) != 0) {
                receive_connection_traffic(connection, server, api_catalogs->native_operations,
                                           api_catalogs->stock_commands, control_command_sequence);
            }
            if (connection.closure != ConnectionClosure::immediate &&
                (connection_events & POLLOUT) != 0) {
                flush_connection_output(connection);
            }
            if ((connection_events & (POLLHUP | POLLERR | POLLNVAL)) != 0) {
                connection.closure = ConnectionClosure::immediate;
            }
        }
        for (std::size_t i = 0; i < pane_pty_routes.size(); ++i) {
            const auto pane_events = poll_descriptors[i + 3 + connection_count].revents;
            if ((pane_events & (POLLIN | POLLOUT | POLLHUP | POLLERR)) != 0) {
                server.advance_pane_pty_io(pane_pty_routes[i].first);
            }
        }
        const auto client_offset = 3 + connection_count + pane_pty_routes.size();
        for (std::size_t i = 0; i < client_routes.size(); ++i) {
            if (ClientTerminalRouting::route_ready_io(
                    server, client_routes[i].client_id, api_catalogs->key_tables,
                    poll_descriptors[client_offset + 2 * i].revents,
                    poll_descriptors[client_offset + 2 * i + 1].revents) ==
                ClientTerminalRouteOutcome::failed) {
                close_stock_client_transport(connections, client_routes[i].client_id);
            }
        }
        const auto control_offset = client_offset + 2 * client_routes.size();
        for (std::size_t i = 0; i < control_routes.size(); ++i) {
            auto &connection = connections[control_routes[i].connection_index];
            if (connection.closure == ConnectionClosure::immediate) {
                continue;
            }
            auto *client_protocol =
                std::get_if<tmux_wire::AcceptedClientProtocol>(&connection.protocol_state);
            if (!client_protocol) {
                connection.closure = ConnectionClosure::immediate;
                continue;
            }
            const short input_events = poll_descriptors[control_offset + 2 * i].revents;
            const short output_events = poll_descriptors[control_offset + 2 * i + 1].revents;
            const auto readiness = tmux_wire::AcceptedClientProtocol::ControlModeReadiness{
                .input_ready = (input_events & POLLIN) != 0,
                .input_closed = (input_events & POLLHUP) != 0,
                .output_ready = (output_events & POLLOUT) != 0,
                .output_closed = (output_events & POLLHUP) != 0,
                .descriptor_failed = ((input_events | output_events) & (POLLERR | POLLNVAL)) != 0,
            };
            apply_stock_transport_action(
                connection, client_protocol->advance_control_mode(readiness, server,
                                                                  api_catalogs->stock_commands,
                                                                  control_command_sequence));
        }
        for (auto &connection : connections) {
            if (auto *native_request = std::get_if<NativeRequestState>(&connection.protocol_state);
                connection.closure != ConnectionClosure::immediate && native_request &&
                native_request->pending_reply) {
                auto completed = native_request->pending_reply->resume(server);
                if (!completed) {
                    queue_native_reply(connection, server, native_request->request_operation,
                                       std::unexpected(completed.error()));
                    native_request->pending_reply.reset();
                } else if (*completed) {
                    queue_native_reply(connection, server, native_request->request_operation,
                                       std::move(**completed));
                    native_request->pending_reply.reset();
                }
            }
        }
        if ((poll_descriptors[0].revents & POLLIN) != 0) {
            accept_connection(stock_listener.descriptor(), AcceptedProtocol::stock_tmux, server,
                              connections);
        }
        if ((poll_descriptors[1].revents & POLLIN) != 0) {
            accept_connection(native_listener.descriptor(), AcceptedProtocol::native_api, server,
                              connections);
        }
    }
    const auto stopped = server.shutdown();
    if (!stopped) {
        std::cerr << stopped.error().message << '\n';
    }
    return stopped ? 0 : 1;
}
} // namespace tmux_cxx
