#include "cli/action_catalog.hpp"
#include "cli/action_registry.hpp"

#include <tmux_cxx/connection/server_connection.hpp>

#include <array>
#include <gtest/gtest.h>
#include <sstream>

namespace tmux_cxx::cli {
namespace {
/** @brief Provide one test-only CLI action whose observable output proves typed invocation. */
struct EchoCliAction {
    /** @brief Own the text decoded from one CLI argument. */
    struct Request {
        std::string text; ///< Copied argument written by the test action.
    };
    static constexpr ActionDescription description{"echo"}; ///< Test-only registered spelling.
    /** @brief Require one argument and copy it before invocation. */
    static Result<Request> read(std::span<const std::string_view> arguments) {
        if (arguments.size() != 1) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "echo requires one argument"});
        }
        return Request{std::string(arguments.front())};
    }
    /** @brief Write the decoded argument through the real CLI invocation boundary. */
    static Result<void> execute(ActionInvocation &invocation, Request request) {
        invocation.output << request.text << '\n';
        return {};
    }
};
} // namespace

/** @brief A registered CLI action owns parsing and execution without a process-entry branch. */
TEST(CliActionRegistry, RegisteredActionOwnsParsingAndExecution) {
    ActionRegistry registry;
    ASSERT_TRUE(registry.add<EchoCliAction>());
    auto actions = std::move(registry).freeze();
    ASSERT_TRUE(actions);
    ServerConnection connection("/unused-by-echo");
    std::ostringstream output;
    ActionInvocation invocation{connection, output};
    const std::array<std::string_view, 2> arguments{"echo", "literal"};
    ASSERT_TRUE(actions->invoke(invocation, arguments));
    EXPECT_EQ(output.str(), "literal\n");
}

/** @brief Duplicate, late, and unknown CLI actions fail without invoking an adapter. */
TEST(CliActionRegistry, ConsumedRegistryAndCatalogRejectInvalidUse) {
    ActionRegistry registry;
    ASSERT_TRUE(registry.add<EchoCliAction>());
    EXPECT_FALSE(registry.add<EchoCliAction>());
    auto actions = std::move(registry).freeze();
    ASSERT_TRUE(actions);
    EXPECT_FALSE(registry.add<EchoCliAction>());
    ServerConnection connection("/unused-by-unknown-action");
    std::ostringstream output;
    ActionInvocation invocation{connection, output};
    const std::array<std::string_view, 1> arguments{"missing"};
    const auto missing = actions->invoke(invocation, arguments);
    ASSERT_FALSE(missing);
    EXPECT_EQ(missing.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_TRUE(output.str().empty());
}

/** @brief The production catalog delegates session CLI syntax to its registered action adapters.
 */
TEST(CliActionCatalog, SessionActionsRejectSyntaxBeforeConnecting) {
    auto actions = build_action_catalog();
    ASSERT_TRUE(actions);
    std::ostringstream usage;
    ASSERT_TRUE(actions->write_usage(usage, "tmux-cxx --socket PATH "));
    EXPECT_EQ(usage.str(), "tmux-cxx --socket PATH create NAME [COMMAND]\n"
                           "tmux-cxx --socket PATH list\n");
    ServerConnection connection("/must-not-be-contacted");
    std::ostringstream output;
    ActionInvocation invocation{connection, output};

    const std::array<std::string_view, 1> incomplete_create{"create"};
    const auto create_failure = actions->invoke(invocation, incomplete_create);
    ASSERT_FALSE(create_failure);
    EXPECT_EQ(create_failure.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(create_failure.error().operation, "create");

    const std::array<std::string_view, 2> excessive_list{"list", "unexpected"};
    const auto list_failure = actions->invoke(invocation, excessive_list);
    ASSERT_FALSE(list_failure);
    EXPECT_EQ(list_failure.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(list_failure.error().operation, "list");
    EXPECT_TRUE(output.str().empty());
}
} // namespace tmux_cxx::cli
