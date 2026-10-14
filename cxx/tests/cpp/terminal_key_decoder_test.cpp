#include "key_table/key_binding_registry.hpp"
#include "protocol/native/operation_registry.hpp"
#include "terminal/terminal_key_decoder.hpp"

#include <tmux_cxx/client/operations/detach_client.hpp>

#include <array>
#include <chrono>
#include <gtest/gtest.h>
#include <string>

namespace tmux_cxx {
namespace {
using namespace std::chrono_literals;
using key_table::KeyCode;
using key_table::KeyModifier;

/** @brief Pair a terminal encoding with its expected normalized key. */
struct EncodedKey {
    std::string_view bytes; ///< Exact encoded terminal input span.
    KeyCode key;            ///< Expected normalized key and modifiers.
};

/** @brief Register an arrow binding whose modifier participates in immutable catalog lookup. */
template <KeyModifier Modifier> struct ArrowBinding {
    static constexpr key_table::KeyBindingDescription description{
        "root",
        {key_table::up_key.value, Modifier},
        "detach-client",
        DetachClient::description.id}; ///< Associate each key variant with a registered operation.
    /** @brief Leave server state unchanged when the test-only binding is invoked. */
    static Result<key_table::KeyBindingEffect> execute(key_table::KeyBindingInvocation &) {
        return key_table::KeyBindingEffect::client_selection_unchanged;
    }
};
} // namespace

/** @brief Fragmentation preserves normalized keys and consumes exactly one original encoding. */
TEST(TerminalKeyDecoder, FragmentedKeysPreserveTheirRawSpans) {
    constexpr std::array keys{
        EncodedKey{"\x1b[A", key_table::up_key},
        EncodedKey{"\x1bOB", key_table::down_key},
        EncodedKey{"\x1b[C", key_table::right_key},
        EncodedKey{"\x1bOD", key_table::left_key},
        EncodedKey{"\x1b[H", key_table::home_key},
        EncodedKey{"\x1bOF", key_table::end_key},
        EncodedKey{"\x1b[5~", key_table::page_up_key},
        EncodedKey{"\x1b[6~", key_table::page_down_key},
        EncodedKey{"\x1b[1;2A", {key_table::up_key.value, KeyModifier::shift}},
        EncodedKey{"\x1b[1;3A", {key_table::up_key.value, KeyModifier::meta}},
        EncodedKey{"\x1b[1;4A", {key_table::up_key.value, KeyModifier::shift | KeyModifier::meta}},
        EncodedKey{"\x1b[1;5A", {key_table::up_key.value, KeyModifier::control}},
        EncodedKey{"\x1b[1;6A",
                   {key_table::up_key.value, KeyModifier::shift | KeyModifier::control}},
        EncodedKey{"\x1b[1;7A",
                   {key_table::up_key.value, KeyModifier::meta | KeyModifier::control}},
        EncodedKey{"\x1b[1;9A", {key_table::up_key.value, KeyModifier::meta}},
        EncodedKey{"\x1b[6;8~",
                   {key_table::page_down_key.value,
                    KeyModifier::shift | KeyModifier::meta | KeyModifier::control}},
        EncodedKey{"\xc3\xa9", KeyCode{0xe9}},
        EncodedKey{"\xe4\xb8\xad", KeyCode{0x4e2d}},
        EncodedKey{"\xf0\x9f\x98\x80", KeyCode{0x1f600}},
        EncodedKey{"\xc2\x80", KeyCode{0x80}},
        EncodedKey{"\xe0\xa0\x80", KeyCode{0x800}},
        EncodedKey{"\xf0\x90\x80\x80", KeyCode{0x10000}},
        EncodedKey{"\xf4\x8f\xbf\xbf", KeyCode{0x10ffff}},
        EncodedKey{"\x1bx", {'x', KeyModifier::meta}},
        EncodedKey{"\x1b\xc3\xa9", {0xe9, KeyModifier::meta}},
        EncodedKey{"\x1b\x1b[A", {key_table::up_key.value, KeyModifier::meta}},
    };
    const auto start = TerminalKeyDecoder::Clock::time_point{};
    for (const auto &encoded : keys) {
        SCOPED_TRACE(std::string(encoded.bytes));
        for (std::size_t split = 1; split < encoded.bytes.size(); ++split) {
            TerminalKeyDecoder decoder;
            const auto partial = decoder.peek(encoded.bytes.substr(0, split), start);
            ASSERT_EQ(partial.status, TerminalKeyStatus::pending);
            EXPECT_EQ(partial.byte_count, 0U);
            const std::string queued = std::string(encoded.bytes) + "\x02x";
            const auto decoded = decoder.peek(queued, start + 1ms);
            ASSERT_EQ(decoded.status, TerminalKeyStatus::decoded);
            ASSERT_TRUE(decoded.key);
            EXPECT_EQ(*decoded.key, encoded.key);
            EXPECT_EQ(decoded.byte_count, encoded.bytes.size());
            EXPECT_EQ(std::string_view(queued).substr(0, decoded.byte_count), encoded.bytes);
            EXPECT_FALSE(decoder.next_deadline());
            decoder.consume();
            const auto suffix =
                decoder.peek(std::string_view(queued).substr(decoded.byte_count), start + 1ms);
            ASSERT_TRUE(suffix.key);
            EXPECT_EQ(*suffix.key, key_table::control_key('b'));
            EXPECT_EQ(suffix.byte_count, 1U);
        }
    }
}

/** @brief One monotonic deadline survives fragments and completed keys remain stable until
 * consumed. */
TEST(TerminalKeyDecoder, DeadlineAndCompletedKeySurviveBackpressure) {
    const auto start = TerminalKeyDecoder::Clock::time_point{};
    TerminalKeyDecoder decoder;
    EXPECT_EQ(decoder.peek("\x1b", start).status, TerminalKeyStatus::pending);
    ASSERT_EQ(decoder.next_deadline(), start + 10ms);
    EXPECT_EQ(decoder.peek("\x1b[", start + 9ms).status, TerminalKeyStatus::pending);
    EXPECT_EQ(decoder.next_deadline(), start + 10ms);
    const auto expired = decoder.peek("\x1b[", start + 10ms);
    ASSERT_TRUE(expired.key);
    EXPECT_EQ(*expired.key, key_table::escape_key);
    EXPECT_EQ(expired.byte_count, 1U);
    EXPECT_FALSE(decoder.next_deadline());
    const auto held = decoder.peek("\x1b[A\x02x", start + 20ms);
    EXPECT_EQ(held.key, expired.key);
    EXPECT_EQ(held.byte_count, expired.byte_count);
    EXPECT_FALSE(decoder.next_deadline());
    decoder.consume();
    EXPECT_EQ(decoder.peek("[A\x02x", start + 20ms).key, KeyCode{'['});

    decoder.consume();
    EXPECT_EQ(decoder.peek("\xe2", start).status, TerminalKeyStatus::pending);
    EXPECT_EQ(decoder.next_deadline(), start + 10ms);
    const auto truncated = decoder.peek("\xe2", start + 10ms);
    EXPECT_EQ(truncated.status, TerminalKeyStatus::unsupported);
    EXPECT_EQ(truncated.byte_count, 1U);
    EXPECT_FALSE(decoder.next_deadline());

    decoder.consume();
    constexpr std::string_view escaped_meta_arrow{"\x1b\x1b[1;3A"};
    const auto literal = decoder.peek(escaped_meta_arrow, start);
    EXPECT_EQ(literal.key, key_table::escape_key);
    EXPECT_EQ(literal.byte_count, 1U);
    decoder.consume();
    const auto meta_arrow = decoder.peek(escaped_meta_arrow.substr(literal.byte_count), start);
    EXPECT_EQ(meta_arrow.key, (KeyCode{key_table::up_key.value, KeyModifier::meta}));
    EXPECT_EQ(meta_arrow.byte_count, escaped_meta_arrow.size() - literal.byte_count);
}

/** @brief Invalid Unicode and unsupported bounded escapes preserve subsequent C0 controls. */
TEST(TerminalKeyDecoder, InvalidInputMakesBoundedProgressBeforeControls) {
    const std::array invalid{
        std::string("\xc0\xaf"),
        std::string("\xe0\x80\x80"),
        std::string("\xf0\x80\x80\x80"),
        std::string("\xed\xa0\x80"),
        std::string("\xf4\x90\x80\x80"),
        std::string("\xe2\x02x"),
        std::string("\x1b[~\x02x"),
        std::string("\x1bO~\x02x"),
        std::string("\x1b[99~\x02x"),
        std::string("\x1b[1;0A\x02x"),
        std::string("\x1b[1;999A\x02x"),
        std::string("\x1b[123\x02x"),
        std::string("\x1b[") + std::string(100, '1') + "\x02x",
    };
    const auto now = TerminalKeyDecoder::Clock::time_point{};
    for (const auto &bytes : invalid) {
        SCOPED_TRACE(bytes);
        TerminalKeyDecoder decoder;
        const auto decoded = decoder.peek(bytes, now);
        EXPECT_EQ(decoded.status, TerminalKeyStatus::unsupported);
        EXPECT_FALSE(decoded.key);
        EXPECT_GT(decoded.byte_count, 0U);
        EXPECT_LE(decoded.byte_count, TerminalKeyDecoder::maximum_key_bytes);
        const auto prefix = bytes.find('\x02');
        if (prefix != std::string::npos) {
            EXPECT_LE(decoded.byte_count, prefix);
        }
        EXPECT_FALSE(decoder.next_deadline());
    }
}

/** @brief Catalog lookup distinguishes modified and unmodified bindings with the same key value. */
TEST(KeyBindingRegistry, ModifiersRemainPartOfCatalogLookup) {
    protocol::native::OperationRegistry operations;
    ASSERT_TRUE(operations.add<DetachClient>());
    auto operation_catalog = std::move(operations).freeze();
    ASSERT_TRUE(operation_catalog);
    key_table::KeyBindingRegistry bindings;
    ASSERT_TRUE(bindings.add<ArrowBinding<KeyModifier::control>>());
    ASSERT_TRUE(bindings.add<ArrowBinding<KeyModifier::none>>());
    ASSERT_TRUE(bindings.add<ArrowBinding<KeyModifier::meta>>());
    auto tables = std::move(bindings).freeze(*operation_catalog);
    ASSERT_TRUE(tables);
    EXPECT_TRUE(tables->contains("root", key_table::up_key));
    EXPECT_TRUE(tables->contains("root", {key_table::up_key.value, KeyModifier::meta}));
    EXPECT_TRUE(tables->contains("root", {key_table::up_key.value, KeyModifier::control}));
    EXPECT_FALSE(tables->contains("root", {key_table::up_key.value, KeyModifier::shift}));
}
} // namespace tmux_cxx
