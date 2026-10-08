#include "terminal/terminal_state.hpp"

#include <gtest/gtest.h>

namespace tmux_cxx::terminal {
/** @brief Partial control and Unicode sequences retain parser state and preserve cell widths and
 * attributes. */
TEST(TerminalState, ChunkBoundariesPreserveMeaning) {
    TerminalState terminal({3, 8});
    ASSERT_TRUE(terminal.ingest_program_output("\x1b[2;"));
    ASSERT_TRUE(terminal.ingest_program_output("3H\x1b[31;1m\xe7\x95"));
    ASSERT_TRUE(terminal.ingest_program_output("\x8c"));
    const auto screen = terminal.snapshot();
    ASSERT_EQ(screen.cells.size(), 24);
    EXPECT_EQ(screen.cells[10].text, U"界");
    EXPECT_EQ(screen.cells[10].width, 2);
    EXPECT_TRUE(screen.cells[10].bold);
    EXPECT_EQ(screen.cells[10].foreground.kind, TerminalColorKind::indexed);
    EXPECT_EQ(screen.cells[10].foreground.value, 1);
    EXPECT_EQ(screen.cursor, (CellPosition{1, 4}));
}

/** @brief UTF-8 continuation survives ASCII-led chunks and malformed sequences independently of
 * delivery boundaries. */
TEST(TerminalState, Utf8ContinuationHasOneOwner) {
    for (const std::string_view bytes : {"A\xe7\x95\x8cZ", "A\xe7.\x81\x1b[31mX\x1b[0m"}) {
        TerminalState whole({4, 8});
        ASSERT_TRUE(whole.ingest_program_output(bytes));
        const auto expected = whole.snapshot();
        for (std::size_t chunk_size = 1; chunk_size < bytes.size(); ++chunk_size) {
            TerminalState fragmented({4, 8});
            for (std::size_t offset = 0; offset < bytes.size(); offset += chunk_size) {
                ASSERT_TRUE(fragmented.ingest_program_output(bytes.substr(offset, chunk_size)));
            }
            EXPECT_TRUE(fragmented.snapshot().cells == expected.cells);
            EXPECT_EQ(fragmented.snapshot().cursor, expected.cursor);
        }
    }
}

/** @brief Malformed UTF-8 cannot make designated character sets depend on output chunk boundaries.
 */
TEST(TerminalState, MalformedUtf8PreservesDesignatedCharsetAcrossChunks) {
    constexpr std::string_view output{"\x1b\xff\xff(0\xffr", 7};
    TerminalState whole({4, 8});
    TerminalState fragmented({4, 8});
    ASSERT_TRUE(whole.ingest_program_output(output));
    ASSERT_TRUE(fragmented.ingest_program_output(output.substr(0, 3)));
    ASSERT_TRUE(fragmented.ingest_program_output(output.substr(3, 3)));
    ASSERT_TRUE(fragmented.ingest_program_output(output.substr(6)));
    const auto expected = whole.snapshot();
    const auto actual = fragmented.snapshot();
    EXPECT_EQ(actual.cells, expected.cells);
    ASSERT_GE(expected.cells.size(), 2);
    EXPECT_EQ(expected.cells[1].text, U"\u23bc");
}

/** @brief Split combining marks attach to narrow or wide rightmost glyphs before following output
 * wraps. */
TEST(TerminalState, RightEdgeCombiningPreservesPendingWrap) {
    for (const std::string_view bytes :
         {"1234567e\xcc\x81\x1b[31mX", "123456\xe7\x95\x8c\xcc\x81\x1b[31mX"}) {
        TerminalState whole({4, 8});
        TerminalState fragmented({4, 8});
        ASSERT_TRUE(whole.ingest_program_output(bytes));
        for (const char byte : bytes) {
            ASSERT_TRUE(fragmented.ingest_program_output(std::string_view(&byte, 1)));
        }
        const auto screen = fragmented.snapshot();
        EXPECT_TRUE(screen.cells == whole.snapshot().cells);
        EXPECT_EQ(screen.cursor, (CellPosition{1, 1}));
        EXPECT_EQ(screen.cells[8].text, U"X");
    }
}

/** @brief Oversized CSI numbers and argument lists remain bounded across delivery fragments. */
TEST(TerminalState, CsiArgumentsStayWithinParserStorage) {
    const std::string numbers = "\x1b[" + std::string(100, '9') + "mZ";
    std::string arguments = "\x1b[";
    for (int argument = 0; argument < 24; ++argument) {
        arguments += "0;";
    }
    arguments += "0mZ";
    for (const auto &bytes : {numbers, arguments}) {
        TerminalState whole({4, 8});
        TerminalState fragmented({4, 8});
        ASSERT_TRUE(whole.ingest_program_output(bytes));
        for (const char byte : bytes) {
            ASSERT_TRUE(fragmented.ingest_program_output(std::string_view(&byte, 1)));
        }
        EXPECT_TRUE(fragmented.snapshot().cells == whole.snapshot().cells);
        EXPECT_EQ(fragmented.snapshot().cursor, (CellPosition{0, 1}));
        EXPECT_EQ(fragmented.snapshot().cells[0].text, U"Z");
    }
}

/** @brief UTF-8 encoded C1 controls consume no cells and leave later erasures within the screen. */
TEST(TerminalState, EncodedControlCodepointKeepsCursorInBounds) {
    TerminalState terminal({4, 8});
    ASSERT_TRUE(terminal.ingest_program_output("A\xc2\x81"));
    auto screen = terminal.snapshot();
    EXPECT_EQ(screen.cursor, (CellPosition{0, 1}));
    EXPECT_EQ(screen.cells[0].text, U"A");

    ASSERT_TRUE(terminal.ingest_program_output("\x1b[31X"));
    screen = terminal.snapshot();
    EXPECT_EQ(screen.cursor, (CellPosition{0, 1}));
    EXPECT_EQ(screen.cells[0].text, U"A");
}

/** @brief Oversized forward cursor movements clamp to the active screen edge without signed
 * overflow. */
TEST(TerminalState, OversizedForwardMovementClampsToScreen) {
    TerminalState vertical({4, 8});
    ASSERT_TRUE(vertical.ingest_program_output("\x1b[4;1H\x1b[2147483646B"));
    EXPECT_EQ(vertical.snapshot().cursor, (CellPosition{3, 0}));

    TerminalState horizontal({4, 8});
    ASSERT_TRUE(horizontal.ingest_program_output("\x1b[1;8H\x1b[2147483646C"));
    EXPECT_EQ(horizontal.snapshot().cursor, (CellPosition{0, 7}));
}

/** @brief Oversized OSC command numbers stay unsupported and preserve following terminal text. */
TEST(TerminalState, OversizedOscCommandStaysUnknown) {
    TerminalState terminal({4, 8});
    ASSERT_TRUE(terminal.ingest_program_output("\x1b]2147483648;ignored\x07Z"));
    const auto screen = terminal.snapshot();
    EXPECT_EQ(screen.cursor, (CellPosition{0, 1}));
    EXPECT_EQ(screen.cells[0].text, U"Z");
}

/** @brief REP without a preceding graphic character is ignored instead of looping forever. */
TEST(TerminalState, RepeatWithoutGlyphIsIgnored) {
    TerminalState terminal({4, 8});
    ASSERT_TRUE(terminal.ingest_program_output("\x1b[2147483646b"));
    EXPECT_EQ(terminal.snapshot().cursor, (CellPosition{0, 0}));
}

/** @brief Cursor queries observe preceding output and queue replies only after complete query
 * sequences. */
TEST(TerminalState, QueriesPreserveStreamOrder) {
    TerminalState terminal({3, 8});
    ASSERT_TRUE(terminal.ingest_program_output("abc\x1b[6"));
    EXPECT_TRUE(terminal.take_program_replies().empty());
    ASSERT_TRUE(terminal.ingest_program_output("n\rZ\x1b[6n"));
    EXPECT_EQ(terminal.take_program_replies(), "\x1b[1;4R\x1b[1;2R");
    EXPECT_TRUE(terminal.take_program_replies().empty());
}

/** @brief Alternate screens restore primary content while bounded history and resize retain
 * explicit state. */
TEST(TerminalState, ScreensHistoryAndResizeHaveSeparateState) {
    TerminalState terminal({2, 5}, 1);
    ASSERT_TRUE(terminal.ingest_program_output("one\r\ntwo\r\nthree\r\nfour"));
    auto screen = terminal.snapshot();
    EXPECT_EQ(screen.history_lines, 1);
    EXPECT_TRUE(screen.history_gap);
    EXPECT_FALSE(screen.alternate_screen);
    const auto primary = screen.cells;
    ASSERT_TRUE(terminal.ingest_program_output("\x1b[?1049h\x1b[2Jalt"));
    EXPECT_TRUE(terminal.snapshot().alternate_screen);
    ASSERT_TRUE(terminal.ingest_program_output("\x1b[?1049l"));
    EXPECT_EQ(terminal.snapshot().cells, primary);
    ASSERT_TRUE(terminal.resize({3, 8}));
    EXPECT_EQ(terminal.snapshot().size, (CellSize{3, 8}));
    EXPECT_EQ(terminal.snapshot().cells.size(), 24);
    const auto revision = terminal.snapshot().revision;
    const auto invalid = terminal.resize({0, 8});
    ASSERT_FALSE(invalid);
    EXPECT_EQ(invalid.error().code, TmuxErrorCode::invalid_argument);
    EXPECT_EQ(terminal.snapshot().revision, revision);
}
/** @brief Excessive query replies mark incomplete state and close further parser admission while
 * preserving the bounded reply prefix. */
TEST(TerminalState, ReplyCapacityFailureIsExplicit) {
    TerminalState terminal({2, 5});
    std::string queries;
    for (int query = 0; query < 15000; ++query) {
        queries += "\x1b[6n";
    }
    const auto overflow = terminal.ingest_program_output(queries);
    ASSERT_FALSE(overflow);
    EXPECT_EQ(overflow.error().code, TmuxErrorCode::protocol);
    EXPECT_FALSE(terminal.snapshot().complete);
    const auto revision = terminal.snapshot().revision;
    EXPECT_FALSE(terminal.ingest_program_output("changed"));
    EXPECT_EQ(terminal.snapshot().revision, revision);
    EXPECT_LE(terminal.take_program_replies().size(), 65536);
}
} // namespace tmux_cxx::terminal
