#include "protocol/native/framing.hpp"
#include "protocol/native/operation_registry.hpp"

#include <tmux_cxx/server/server.hpp>

#include <array>
#include <gtest/gtest.h>

namespace tmux_cxx {
namespace native = protocol::native;
namespace {
/** @brief Register an independent rename entry without adding transport branches. */
struct RegisteredRename {
    /** @brief Own the session identity and replacement name for the test operation. */
    struct Request {
        SessionId id;     ///< Identity resolved by shared rename validation.
        std::string name; ///< Replacement session name.
    };
    using Response = void; ///< Success carries no response payload.
    /** @brief Stable native identity reserved for this registered test extension. */
    static constexpr native::OperationDescription description{
        native::OperationId{4096}, "session.test_rename", native::OperationEffect::mutation};
    /** @brief Decode a session identity and replacement name without mutating state. */
    static Request read(native::NativePayloadReader &reader) {
        return {SessionId{reader.u64()}, reader.string()};
    }
    /** @brief Rename through the server's normal validation and mutation path. */
    static Result<void> execute(Server &server, Request request) {
        return server.rename_session(request.id, std::move(request.name));
    }
    /** @brief Return the empty successful mutation payload. */
    static native::OperationReply write() { return native::Bytes{}; }
};
/** @brief Present valid operation types with an undeclared native capability. */
struct UnknownCapabilityRename : RegisteredRename {
    static constexpr native::OperationDescription description{
        native::OperationId{4097}, "session.unknown_capability", native::OperationEffect::mutation,
        1ULL << 63}; ///< Controlled invalid registration metadata.
};
/** @brief Present a valid operation shape with an undeclared effect classification. */
struct UnknownEffectRename : RegisteredRename {
    static constexpr native::OperationDescription description{
        native::OperationId{4098}, "session.unknown_effect",
        static_cast<native::OperationEffect>(255)}; ///< Controlled invalid effect metadata.
};
/** @brief Reuse a rename identity under a distinct operation name. */
struct RepeatedRenameId : RegisteredRename {
    static constexpr native::OperationDescription description{
        native::OperationId{4096}, "session.other_rename",
        native::OperationEffect::mutation}; ///< Collision only in the stable wire identity.
};
/** @brief Reuse a rename name under a distinct wire identity. */
struct RepeatedRenameName : RegisteredRename {
    static constexpr native::OperationDescription description{
        native::OperationId{4099}, "session.test_rename",
        native::OperationEffect::mutation}; ///< Collision only in the stable operation name.
};
} // namespace

/** @brief A newly registered typed operation executes without transport-specific dispatch changes.
 */
TEST(NativeOperationRegistry, RegisteredExtensionMutatesServer) {
    Server server;
    auto session = server.create_session("before", "/bin/cat");
    ASSERT_TRUE(session);
    native::OperationRegistry registry;
    ASSERT_TRUE(registry.add<RegisteredRename>());
    auto catalog = std::move(registry).freeze();
    ASSERT_TRUE(catalog);
    native::NativePayloadWriter request;
    request.u64(session->id.value);
    request.string("after");
    auto response = catalog->invoke(RegisteredRename::description.id, server, request.bytes);
    ASSERT_TRUE(response);
    EXPECT_TRUE(std::holds_alternative<native::Bytes>(*response));
    EXPECT_EQ(server.sessions().front().name, "after");
}

/** @brief Trailing or unknown request data is rejected before the registered mutation executes. */
TEST(NativeOperationRegistry, MalformedRequestsPreserveServerState) {
    Server server;
    auto session = server.create_session("before", "/bin/cat");
    ASSERT_TRUE(session);
    native::OperationRegistry registry;
    ASSERT_TRUE(registry.add<RegisteredRename>());
    auto catalog = std::move(registry).freeze();
    ASSERT_TRUE(catalog);
    native::NativePayloadWriter request;
    request.u64(session->id.value);
    request.string("after");
    request.u16(7);
    auto malformed = catalog->invoke(RegisteredRename::description.id, server, request.bytes);
    ASSERT_FALSE(malformed);
    EXPECT_EQ(malformed.error().code, TmuxErrorCode::protocol);
    auto unknown = catalog->invoke(native::OperationId{65535}, server, {});
    ASSERT_FALSE(unknown);
    EXPECT_EQ(unknown.error().code, TmuxErrorCode::protocol);
    EXPECT_EQ(server.sessions().front().name, "before");
}

/** @brief Duplicate identities and registration after freezing fail without changing the catalog.
 */
TEST(NativeOperationRegistry, DuplicateAndLateRegistrationFail) {
    native::OperationRegistry registry;
    ASSERT_TRUE(registry.add<RegisteredRename>());
    EXPECT_FALSE(registry.add<RegisteredRename>());
    EXPECT_FALSE(registry.add<RepeatedRenameId>());
    EXPECT_FALSE(registry.add<RepeatedRenameName>());
    auto catalog = std::move(registry).freeze();
    ASSERT_TRUE(catalog);
    EXPECT_FALSE(registry.add<RegisteredRename>());
    native::OperationRegistry invalid;
    ASSERT_TRUE(invalid.add<UnknownCapabilityRename>());
    EXPECT_FALSE(std::move(invalid).freeze());
    native::OperationRegistry invalid_effect;
    ASSERT_TRUE(invalid_effect.add<UnknownEffectRename>());
    EXPECT_FALSE(std::move(invalid_effect).freeze());
}

/** @brief Missing or changed public operation descriptions prevent lookup from freezing. */
TEST(NativeOperationRegistry, PublicContractMustMatchRegisteredOperations) {
    const std::array mismatches{
        native::OperationDescription{native::OperationId{4097}, "session.test_rename",
                                     native::OperationEffect::mutation},
        native::OperationDescription{native::OperationId{4096}, "session.changed_name",
                                     native::OperationEffect::mutation},
        native::OperationDescription{native::OperationId{4096}, "session.test_rename",
                                     native::OperationEffect::query},
        native::OperationDescription{native::OperationId{4096}, "session.test_rename",
                                     native::OperationEffect::mutation, 1},
    };
    for (const auto &description : mismatches) {
        native::OperationRegistry candidate;
        ASSERT_TRUE(candidate.add<RegisteredRename>());
        const std::array required{description};
        EXPECT_FALSE(std::move(candidate).freeze(required));
    }
    native::OperationRegistry matching;
    ASSERT_TRUE(matching.add<RegisteredRename>());
    const std::array required{RegisteredRename::description};
    auto matching_catalog = std::move(matching).freeze(required);
    ASSERT_TRUE(matching_catalog);
    EXPECT_TRUE(matching_catalog->contains(RegisteredRename::description.id));
    native::OperationRegistry duplicated;
    ASSERT_TRUE(duplicated.add<RegisteredRename>());
    const std::array repeated{RegisteredRename::description, RegisteredRename::description};
    EXPECT_FALSE(std::move(duplicated).freeze(repeated));
}
} // namespace tmux_cxx
