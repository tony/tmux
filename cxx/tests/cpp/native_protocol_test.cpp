#include "connection/native_request_transport.hpp"
#include "platform/posix/owned_fd.hpp"
#include "protocol/native/handshake.hpp"
#include "protocol/native/operation_registry.hpp"
#include "protocol/native/reply_envelope.hpp"
#include "protocol/native/server_request.hpp"
#include "protocol/native/session_event_codec.hpp"
#include "text/utf8.hpp"
#include <tmux_cxx/server/server.hpp>
#include <tmux_cxx/session/operations/create_session.hpp>

#include <array>
#include <cerrno>
#include <chrono>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <future>
#include <gtest/gtest.h>
#include <semaphore>
#include <stop_token>
#include <string>
#include <sys/socket.h>
#include <sys/un.h>
#include <thread>
#include <unistd.h>

namespace tmux_cxx {
namespace native = protocol::native;
namespace {
/** @brief Remove one test-owned Unix socket after its listener closes. */
struct OwnedSocketPath {
    /** @brief Unlink only the path created by this test process. */
    ~OwnedSocketPath() { (void)::unlink(value.c_str()); }
    std::string value; ///< Unique native endpoint owned by this test.
};
} // namespace
/** @brief Native envelopes own successful bytes and retain declared failures with validated server
 * ownership. */
TEST(NativeProtocol, ReplyEnvelopePreservesResultOwnership) {
    const std::string instance(32, 'a');
    auto bytes = native::encode_reply_envelope(instance, native::Bytes{0, 255});
    auto success = native::decode_reply_envelope(bytes);
    ASSERT_TRUE(success);
    ASSERT_TRUE(success->operation_result);
    bytes.back() = 0;
    EXPECT_EQ(*success->operation_result, (native::Bytes{0, 255}));
    EXPECT_EQ(success->server_instance, instance);
    bytes = native::encode_reply_envelope(
        instance, std::unexpected(TmuxError{TmuxErrorCode::not_found, "missing pane"}));
    auto failure = native::decode_reply_envelope(bytes);
    ASSERT_TRUE(failure);
    ASSERT_FALSE(failure->operation_result);
    EXPECT_EQ(failure->operation_result.error().code, TmuxErrorCode::not_found);
    EXPECT_EQ(failure->operation_result.error().server_instance, instance);
    bytes.push_back(0);
    EXPECT_FALSE(native::decode_reply_envelope(bytes));
    EXPECT_THROW(native::encode_reply_envelope("", native::Bytes{}), std::invalid_argument);
}
/** @brief Native negotiation describes the frozen catalog before any session effects. */
TEST(NativeProtocol, HandshakeDeclaresVersionCapabilitiesAndOperations) {
    Server server;
    native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<CreateSession>());
    auto catalog = std::move(operations).freeze();
    ASSERT_TRUE(catalog);
    auto request = native::handshake_request(native::protocol_version);
    auto reply = native::invoke_request(request, server, *catalog);
    ASSERT_TRUE(reply);
    ASSERT_TRUE(std::holds_alternative<native::Bytes>(*reply));
    auto negotiated = native::decode_handshake(std::get<native::Bytes>(*reply));
    ASSERT_TRUE(negotiated);
    EXPECT_EQ(negotiated->version, native::protocol_version);
    EXPECT_EQ(negotiated->max_payload, native::max_payload);
    ASSERT_EQ(negotiated->operations.size(), 1);
    EXPECT_EQ(negotiated->operations.front().id, CreateSession::description.id);
    EXPECT_EQ(negotiated->operations.front().name, "session.create");
    EXPECT_NE(negotiated->capabilities & CreateSession::description.required_capabilities, 0);
    EXPECT_TRUE(server.sessions().empty());
}
/** @brief Unsupported negotiation and missing server identity reject requests before mutation. */
TEST(NativeProtocol, NegotiationFailureCannotCreateSession) {
    Server server;
    native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<CreateSession>());
    auto catalog = std::move(operations).freeze();
    ASSERT_TRUE(catalog);
    auto version = native::invoke_request(native::handshake_request(65535), server, *catalog);
    ASSERT_FALSE(version);
    EXPECT_EQ(version.error().code, TmuxErrorCode::protocol);
    native::NativePayloadWriter payload;
    payload.string("");
    payload.string("forbidden");
    payload.string("/bin/cat");
    auto missing_identity = native::invoke_request(
        {CreateSession::description.id, std::move(payload.bytes)}, server, *catalog);
    ASSERT_FALSE(missing_identity);
    EXPECT_EQ(missing_identity.error().code, TmuxErrorCode::protocol);
    EXPECT_TRUE(server.sessions().empty());
}
/** @brief Handshake decoding rejects trailing bytes and undeclared capability references. */
TEST(NativeProtocol, MalformedAdvertisementIsRejected) {
    auto valid = native::encode_handshake(std::array{CreateSession::description});
    ASSERT_TRUE(native::decode_handshake(valid));
    valid.push_back(0);
    EXPECT_FALSE(native::decode_handshake(valid));
    native::NativePayloadWriter payload;
    payload.u16(native::protocol_version);
    payload.u32(native::max_payload);
    payload.u64(1ULL << 63);
    payload.u32(0);
    auto decoded = native::decode_handshake(payload.bytes);
    ASSERT_FALSE(decoded);
    EXPECT_EQ(decoded.error().code, TmuxErrorCode::protocol);
}
/** @brief Native client snapshots preserve an attached control route without terminal dimensions.
 */
TEST(NativeProtocol, AttachedControlClientHasNoTerminalDimensions) {
    native::NativePayloadWriter writer;
    writer.client({ClientId{2}, SessionId{3}, "", true, false, 0, 0, 4});
    native::NativePayloadReader reader(writer.bytes);

    const auto client = reader.client();
    EXPECT_EQ(client.id, ClientId{2});
    EXPECT_EQ(client.session, SessionId{3});
    EXPECT_TRUE(client.identified);
    EXPECT_FALSE(client.has_terminal);
    EXPECT_EQ(client.rows, 0U);
    EXPECT_EQ(client.columns, 0U);
    EXPECT_TRUE(reader.finished());
}
/** @brief Native event replies must continue the requested owner-qualified sequence exactly. */
TEST(NativeProtocol, SessionEventBatchRequiresRequestedContinuity) {
    const std::string instance(32, 'a');
    const SessionEventCursor requested{instance, 7};
    const SessionClosedEvent event{SessionId{1}, "closed", 1};
    const std::array invalid_batches{
        SessionEventBatch{{{8, event}}, {instance, 9}},
        SessionEventBatch{{{7, event}, {9, event}}, {instance, 10}},
        SessionEventBatch{{{7, event}}, {instance, 9}},
        SessionEventBatch{{}, {std::string(32, 'b'), 7}},
    };

    for (const auto &batch : invalid_batches) {
        native::NativePayloadWriter writer;
        native::write_session_event_batch(writer, batch);
        native::NativePayloadReader reader(writer.bytes);
        EXPECT_THROW((void)native::read_session_event_batch(reader, requested),
                     std::invalid_argument);
    }
}
/** @brief Cancellation interrupts an admitted native receive without becoming peer closure. */
TEST(NativeProtocol, AdmittedRequestCancellationInterruptsReceive) {
    const auto server_socket =
        std::filesystem::temp_directory_path() /
        ("tmux-cxx-cancel-" + std::to_string(static_cast<long long>(::getpid())));
    OwnedSocketPath native_socket{server_socket.string() + ".native"};
    posix::OwnedFd listener(::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0));
    ASSERT_GE(listener.get(), 0);
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    ASSERT_LT(native_socket.value.size(), sizeof(address.sun_path));
    std::memcpy(address.sun_path, native_socket.value.c_str(), native_socket.value.size() + 1);
    ASSERT_EQ(::bind(listener.get(), reinterpret_cast<const sockaddr *>(&address), sizeof(address)),
              0);
    ASSERT_EQ(::listen(listener.get(), 1), 0);

    std::promise<std::string> admission;
    auto admitted = admission.get_future();
    std::binary_semaphore close_peer(0);
    /** @brief Admit one request byte, then keep the peer open until the assertion completes. */
    auto hold_reply = [&] {
        posix::OwnedFd accepted(::accept(listener.get(), nullptr, nullptr));
        if (accepted.get() < 0) {
            const int failure = errno;
            admission.set_value(std::strerror(failure));
            return;
        }
        std::uint8_t byte = 0;
        const auto received = ::recv(accepted.get(), &byte, sizeof(byte), 0);
        if (received <= 0) {
            const int failure = errno;
            admission.set_value(received == 0 ? "request closed before admission"
                                              : std::strerror(failure));
            return;
        }
        admission.set_value({});
        close_peer.acquire();
    };
    std::jthread peer(std::move(hold_reply));

    NativeRequestTransport transport(server_socket.string());
    std::stop_source cancellation;
    std::promise<Result<native::Bytes>> completion;
    auto completed = completion.get_future();
    /** @brief Issue one request whose fake peer deliberately withholds a reply. */
    auto await_cancellation = [&] {
        completion.set_value(transport.request(
            native::OperationId{1}, {}, native::Clock::now() + std::chrono::milliseconds(750),
            cancellation.get_token()));
    };
    std::jthread requester(std::move(await_cancellation));

    ASSERT_TRUE(admitted.get().empty());
    EXPECT_TRUE(cancellation.request_stop());
    const auto readiness = completed.wait_for(std::chrono::milliseconds(250));
    close_peer.release();
    EXPECT_EQ(readiness, std::future_status::ready);
    auto result = completed.get();
    ASSERT_FALSE(result);
    EXPECT_EQ(result.error().code, TmuxErrorCode::cancelled);
}
/** @brief Native terminal text preserves valid Unicode and rejects encodings that cannot retain the
 * declared scalar meaning. */
TEST(NativeProtocol, TerminalTextRequiresValidUtf8Scalars) {
    EXPECT_TRUE(text::valid_utf8("界\xf4\x8f\xbf\xbf"));
    for (const std::string_view invalid :
         std::array{"\xc0\x80", "\xe0\x80\x80", "\xf0\x80\x80\x80", "\xed\xa0\x80", "\xe7\x95",
                    "\x80", "\xf4\x90\x80\x80", "\xe2\x41\x41"}) {
        EXPECT_FALSE(text::valid_utf8(invalid));
    }
}
} // namespace tmux_cxx
