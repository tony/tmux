#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

/** @brief Retained terminal cells and parser state independent of PTY lifetime and client
 * viewports. */
namespace tmux_cxx::terminal {
/** @brief Describe terminal dimensions in character cells rather than client or pixel metrics. */
struct CellSize {
    std::uint16_t rows;    ///< Positive terminal row count.
    std::uint16_t columns; ///< Positive terminal column count.
    /** @brief Compare terminal cell dimensions without interpreting viewport policy. */
    bool operator==(const CellSize &) const = default;
};
/** @brief Locate a cursor or cell using zero-based terminal coordinates. */
struct CellPosition {
    std::uint16_t row;    ///< Zero-based terminal row.
    std::uint16_t column; ///< Zero-based terminal column.
    /** @brief Compare zero-based terminal positions. */
    bool operator==(const CellPosition &) const = default;
};
/** @brief Distinguish terminal defaults, palette indices, and explicit RGB colors. */
enum class TerminalColorKind { default_color, indexed, rgb };
/** @brief Preserve symbolic default and palette colors until client rendering resolves them. */
struct TerminalColor {
    TerminalColorKind kind; ///< Color interpretation rather than an assumed client palette.
    std::uint32_t value;    ///< Palette index or packed 0xRRGGBB; zero for a symbolic default.
    /** @brief Compare terminal colors while preserving their symbolic interpretation. */
    bool operator==(const TerminalColor &) const = default;
};
/** @brief Own one terminal cell's Unicode sequence, display width, colors, and rendition. */
struct TerminalCell {
    std::u32string
        text; ///< Owned base and combining code points; empty for a wide-cell continuation.
    std::uint8_t width;         ///< Display columns; zero marks the continuation of a wider cell.
    TerminalColor foreground;   ///< Symbolic foreground color retained for client rendering.
    TerminalColor background;   ///< Symbolic background color retained for client rendering.
    bool bold;                  ///< Bold rendition independent of palette brightening.
    bool italic;                ///< Italic rendition.
    bool blink;                 ///< Blinking rendition.
    bool reverse;               ///< Foreground/background reversal applied during rendering.
    bool conceal;               ///< Concealed glyphs retain their text for terminal state.
    bool strike;                ///< Struck-through rendition.
    std::uint8_t underline;     ///< Zero, single, double, or curly underline encoded as 0..3.
    std::uint8_t font;          ///< Selected terminal font index, 0..9.
    bool double_width;          ///< DEC double-width line rendition.
    std::uint8_t double_height; ///< Zero or the top/bottom half of a DEC double-height line, 1..2.
    bool small;                 ///< Small-character rendition.
    std::uint8_t baseline;      ///< Normal, raised, or lowered baseline encoded as 0..2.
    /** @brief Compare copied cell content and rendition without borrowing terminal storage. */
    bool operator==(const TerminalCell &) const = default;
};
/** @brief Materialize the active terminal screen independently of raw bytes and client delivery
 * state. */
struct TerminalSnapshot {
    CellSize size;          ///< Program terminal dimensions in cells.
    CellPosition cursor;    ///< Zero-based cursor within the active screen.
    bool cursor_visible;    ///< Terminal-requested cursor visibility.
    bool alternate_screen;  ///< The active buffer is the alternate screen.
    bool complete;          ///< False after a contained callback failure; copied cells may then be
                            ///< incomplete.
    std::uint64_t revision; ///< Monotonic parser/resize observation sequence, distinct from server
                            ///< metadata revisions.
    std::vector<TerminalCell>
        cells; ///< Owned row-major visible cells; exactly rows multiplied by columns.
    std::size_t
        history_lines; ///< Retained primary scrollback lines; no raw-output recovery is implied.
    bool history_gap;  ///< An earlier terminal-history prefix has been discarded.
};
} // namespace tmux_cxx::terminal
