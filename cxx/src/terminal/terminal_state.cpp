#include "terminal/terminal_state.hpp"

#include <vterm.h>

#include <algorithm>
#include <array>
#include <deque>
#include <limits>
#include <optional>
#include <stdexcept>
#include <utility>

namespace tmux_cxx::terminal {
namespace {
/** @brief Validate bounded terminal storage before allocation or resize. */
bool valid_size(CellSize size) {
    return size.rows > 0 && size.columns > 0 && size.rows <= 512 && size.columns <= 512 &&
           static_cast<std::size_t>(size.rows) * size.columns <= 16384;
}
/** @brief Translate libvterm color tags without resolving symbolic defaults or palette entries. */
TerminalColor copy_terminal_color(const VTermColor &color) {
    if ((color.type & VTERM_COLOR_DEFAULT_MASK) != 0) {
        return {TerminalColorKind::default_color, 0};
    }
    if (VTERM_COLOR_IS_INDEXED(&color)) {
        return {TerminalColorKind::indexed, color.indexed.idx};
    }
    return {TerminalColorKind::rgb, (static_cast<std::uint32_t>(color.rgb.red) << 16) |
                                        (static_cast<std::uint32_t>(color.rgb.green) << 8) |
                                        color.rgb.blue};
}
/** @brief Copy libvterm cell values without exposing library storage or wide-cell sentinel code
 * points. */
TerminalCell copy_terminal_cell(const VTermScreenCell &cell) {
    std::u32string text;
    const bool continuation = cell.chars[0] == std::numeric_limits<std::uint32_t>::max();
    if (!continuation) {
        for (auto codepoint : cell.chars) {
            if (codepoint == 0) {
                break;
            }
            text.push_back(static_cast<char32_t>(codepoint));
        }
        if (text.empty()) {
            text = U" ";
        }
    }
    return {std::move(text),
            static_cast<std::uint8_t>(continuation ? 0 : cell.width),
            copy_terminal_color(cell.fg),
            copy_terminal_color(cell.bg),
            static_cast<bool>(cell.attrs.bold),
            static_cast<bool>(cell.attrs.italic),
            static_cast<bool>(cell.attrs.blink),
            static_cast<bool>(cell.attrs.reverse),
            static_cast<bool>(cell.attrs.conceal),
            static_cast<bool>(cell.attrs.strike),
            static_cast<std::uint8_t>(cell.attrs.underline),
            static_cast<std::uint8_t>(cell.attrs.font),
            static_cast<bool>(cell.attrs.dwl),
            static_cast<std::uint8_t>(cell.attrs.dhl),
            static_cast<bool>(cell.attrs.small),
            static_cast<std::uint8_t>(cell.attrs.baseline)};
}
/** @brief Free an externally allocated terminal exactly once. */
struct VTermDeleter {
    /** @brief Release the external terminal allocation without invoking user callbacks. */
    void operator()(VTerm *terminal) const noexcept { vterm_free(terminal); }
};
} // namespace

/** @brief Retain external terminal ownership and contain every C callback exception locally. */
struct TerminalState::ParserState {
    /** @brief Retain external cell values together with the row's logical-line boundary. */
    struct HistoryLine {
        std::vector<VTermScreenCell> cells; ///< Cells at their original retained width.
        bool continuation;                  ///< This row continues the previous logical line.
    };
    std::unique_ptr<VTerm, VTermDeleter>
        terminal;                  ///< Exclusive external parser and screen allocation.
    VTermScreen *screen = nullptr; ///< Borrowed screen owned by terminal for this state's lifetime.
    CellSize size;                 ///< Current bounded program terminal dimensions.
    std::size_t history_limit;     ///< Maximum retained primary scrollback lines.
    std::deque<HistoryLine>
        history; ///< Owned scrollback with copied cells and their original widths.
    std::array<bool, 512> outgoing_continuations{}; ///< Pre-move flags for bounded outgoing rows.
    int outgoing_rows = 0; ///< Number of pre-move row flags captured by the external bridge.
    bool outgoing_continuation = false;      ///< Flag prepared for the next history callback.
    bool outgoing_row_prepared = false;      ///< Require metadata before retaining outgoing cells.
    std::optional<bool> popped_continuation; ///< Boundary retained until resize restores the row.
    bool history_gap = false;      ///< Earlier terminal history has been evicted or cleared.
    bool cursor_visible = true;    ///< Terminal cursor visibility set by property callbacks.
    bool alternate_screen = false; ///< Currently selected terminal buffer.
    bool failed = false; ///< Contained allocation or reply-bound failure prevents further mutation.
    std::uint64_t revision =
        0;               ///< Parser/resize observation sequence distinct from server metadata.
    std::string replies; ///< Ordered encoded program replies, bounded to 64 KiB.

    /** @brief Allocate the terminal and bind synchronous callbacks to this nonmoving state. */
    ParserState(CellSize size, std::size_t history_limit)
        : size(size), history_limit(history_limit) {
        if (!valid_size(size) || history_limit > 1000) {
            throw std::invalid_argument("terminal dimensions or history exceed storage bounds");
        }
        terminal.reset(vterm_new(size.rows, size.columns));
        if (!terminal) {
            throw std::bad_alloc();
        }
        vterm_set_utf8(terminal.get(), 1);
        vterm_output_set_callback(terminal.get(), &write_reply, this);
        screen = vterm_obtain_screen(terminal.get());
        vterm_screen_enable_altscreen(screen, 1);
        vterm_screen_enable_reflow(screen, false);
        static const VTermScreenCallbacks callbacks{nullptr,       nullptr,      nullptr,
                                                    &set_property, nullptr,      nullptr,
                                                    &push_history, &pop_history, &clear_history};
        vterm_screen_set_callbacks(screen, &callbacks, this);
        vterm_screen_reset(screen, 1);
    }
    /** @brief Copy ordered program replies; allocation and capacity failures never escape the C
     * callback. */
    static void write_reply(const char *bytes, std::size_t length,
                            void *callback_context) noexcept {
        auto &parser_state = *static_cast<ParserState *>(callback_context);
        if (parser_state.failed) {
            return;
        }
        try {
            if (length > 65536 - parser_state.replies.size()) {
                parser_state.failed = true;
                return;
            }
            parser_state.replies.append(bytes, length);
        } catch (...) {
            parser_state.failed = true;
        }
    }
    /** @brief Track terminal visibility and buffer selection without allocating or invoking
     * external observers. */
    static int set_property(VTermProp property, VTermValue *value,
                            void *callback_context) noexcept {
        auto &parser_state = *static_cast<ParserState *>(callback_context);
        if (property == VTERM_PROP_CURSORVISIBLE) {
            parser_state.cursor_visible = value->boolean;
        } else if (property == VTERM_PROP_ALTSCREEN) {
            parser_state.alternate_screen = value->boolean;
        }
        return 1;
    }
    /** @brief Copy primary scrollback within its line bound; allocation failures mark incomplete
     * state without escaping C. */
    static int push_history(int columns, const VTermScreenCell *cells,
                            void *callback_context) noexcept {
        auto &parser_state = *static_cast<ParserState *>(callback_context);
        if (parser_state.failed) {
            return 0;
        }
        if (!parser_state.outgoing_row_prepared) {
            parser_state.failed = true;
            return 0;
        }
        const auto continuation = parser_state.outgoing_continuation;
        parser_state.outgoing_row_prepared = false;
        try {
            if (parser_state.history_limit == 0) {
                parser_state.history_gap = true;
                return 1;
            }
            parser_state.history.push_back(
                HistoryLine{std::vector<VTermScreenCell>(cells, cells + columns), continuation});
            if (parser_state.history.size() > parser_state.history_limit) {
                parser_state.history.pop_front();
                parser_state.history_gap = true;
            }
            return 1;
        } catch (...) {
            parser_state.failed = true;
            return 0;
        }
    }
    /** @brief Restore the newest scrollback row during resize, padding columns with default cells
     * without allocating. */
    static int pop_history(int columns, VTermScreenCell *cells, void *callback_context) noexcept {
        auto &parser_state = *static_cast<ParserState *>(callback_context);
        if (parser_state.failed || parser_state.history.empty()) {
            return 0;
        }
        VTermScreenCell blank{};
        blank.width = 1;
        vterm_state_get_default_colors(vterm_obtain_state(parser_state.terminal.get()), &blank.fg,
                                       &blank.bg);
        std::fill_n(cells, columns, blank);
        const auto &line = parser_state.history.back();
        std::copy_n(line.cells.begin(),
                    std::min(line.cells.size(), static_cast<std::size_t>(columns)), cells);
        parser_state.popped_continuation = line.continuation;
        parser_state.history.pop_back();
        return 1;
    }
    /** @brief Clear retained terminal history and preserve the fact that earlier lines were
     * discarded. */
    static int clear_history(void *callback_context) noexcept {
        auto &parser_state = *static_cast<ParserState *>(callback_context);
        parser_state.history_gap |= !parser_state.history.empty();
        parser_state.history.clear();
        return 1;
    }
};

/** @brief Bridge generated external call sites to the owning C++ terminal callback context. */
struct VTermHistoryCallbacks {
    /** @brief Borrow the registered screen context without relying on state callback internals.
     *
     * Only the private TerminalState screen installs these hooks and its ParserState callback
     * context. */
    static TerminalState::ParserState *context(VTerm *terminal) noexcept {
        return static_cast<TerminalState::ParserState *>(
            vterm_screen_get_cbdata(vterm_obtain_screen(terminal)));
    }
    /** @brief Preserve outgoing continuation flags before scroll or resize changes line metadata.
     */
    static void prepare_rows(VTerm *terminal, const VTermLineInfo *rows, int count) noexcept {
        auto *state = context(terminal);
        if (!state || state->failed) {
            return;
        }
        if (count < 0 || static_cast<std::size_t>(count) > state->outgoing_continuations.size()) {
            state->failed = true;
            return;
        }
        state->outgoing_rows = count;
        for (int row = 0; row < count; ++row) {
            state->outgoing_continuations[static_cast<std::size_t>(row)] =
                rows && rows[row].continuation;
        }
    }
    /** @brief Pair the next disappearing cell row with its previously captured boundary. */
    static void prepare_row(VTerm *terminal, int row) noexcept {
        auto *state = context(terminal);
        if (!state || state->failed) {
            return;
        }
        if (row < 0 || row >= state->outgoing_rows) {
            state->failed = true;
            return;
        }
        state->outgoing_continuation = state->outgoing_continuations[static_cast<std::size_t>(row)];
        state->outgoing_row_prepared = true;
    }
    /** @brief Transfer the popped history boundary to the external resize destination row. */
    static int popped_continuation(VTerm *terminal) noexcept {
        auto *state = context(terminal);
        if (!state || state->failed) {
            return 0;
        }
        if (!state->popped_continuation) {
            state->failed = true;
            return 0;
        }
        const auto continuation = *state->popped_continuation;
        state->popped_continuation.reset();
        return continuation ? 1 : 0;
    }
};

TerminalState::TerminalState(CellSize size, std::size_t history_limit)
    : parser_state_(std::make_unique<ParserState>(size, history_limit)) {}
TerminalState::~TerminalState() = default;

Result<void> TerminalState::ingest_program_output(std::string_view bytes) {
    if (!parser_state_->failed) {
        if (vterm_input_write(parser_state_->terminal.get(), bytes.data(), bytes.size()) !=
            bytes.size()) {
            parser_state_->failed = true;
        }
        ++parser_state_->revision;
    }
    if (parser_state_->failed) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol,
                      "terminal state is incomplete after a callback or reply-capacity failure"});
    }
    return {};
}

Result<void> TerminalState::resize(CellSize size) {
    if (!valid_size(size)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid terminal cell dimensions"});
    }
    if (parser_state_->failed) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol, "terminal state is incomplete"});
    }
    vterm_set_size(parser_state_->terminal.get(), size.rows, size.columns);
    parser_state_->size = size;
    ++parser_state_->revision;
    if (parser_state_->failed) {
        return std::unexpected(TmuxError{TmuxErrorCode::protocol,
                                         "terminal resize could not preserve complete state"});
    }
    return {};
}

TerminalSnapshot TerminalState::snapshot() const {
    VTermPos cursor{};
    vterm_state_get_cursorpos(vterm_obtain_state(parser_state_->terminal.get()), &cursor);
    TerminalSnapshot terminal_snapshot{
        parser_state_->size,
        {static_cast<std::uint16_t>(cursor.row), static_cast<std::uint16_t>(cursor.col)},
        parser_state_->cursor_visible,
        parser_state_->alternate_screen,
        !parser_state_->failed,
        parser_state_->revision,
        {},
        parser_state_->history.size(),
        parser_state_->history_gap};
    terminal_snapshot.cells.reserve(static_cast<std::size_t>(parser_state_->size.rows) *
                                    parser_state_->size.columns);
    for (int row = 0; row < parser_state_->size.rows; ++row) {
        for (int column = 0; column < parser_state_->size.columns; ++column) {
            VTermScreenCell cell{};
            vterm_screen_get_cell(parser_state_->screen, {row, column}, &cell);
            terminal_snapshot.cells.push_back(copy_terminal_cell(cell));
        }
    }
    return terminal_snapshot;
}

std::uint64_t TerminalState::revision() const noexcept { return parser_state_->revision; }

TerminalHistorySnapshot TerminalState::history_snapshot() const {
    const auto visible = snapshot();
    const auto history_rows = visible.alternate_screen ? 0 : parser_state_->history.size();
    TerminalHistorySnapshot captured{visible.size,
                                     visible.cursor,
                                     visible.cursor_visible,
                                     visible.alternate_screen,
                                     visible.complete,
                                     visible.revision,
                                     static_cast<std::uint32_t>(history_rows),
                                     !visible.alternate_screen && visible.history_gap,
                                     {}};
    captured.lines.reserve(history_rows + visible.size.rows);
    if (!visible.alternate_screen) {
        for (const auto &stored : parser_state_->history) {
            TerminalLine line{{}, stored.continuation};
            line.cells.reserve(stored.cells.size());
            for (std::size_t column = 0; column < stored.cells.size(); ++column) {
                const auto &cell = stored.cells[column];
                if (cell.chars[0] != 0) {
                    line.used_columns = static_cast<std::uint32_t>(column + 1);
                }
                line.cells.push_back(copy_terminal_cell(cell));
            }
            captured.lines.push_back(std::move(line));
        }
    }
    const auto *state = vterm_obtain_state(parser_state_->terminal.get());
    for (std::size_t row = 0; row < visible.size.rows; ++row) {
        TerminalLine line{
            {},
            static_cast<bool>(
                vterm_state_get_lineinfo(state, static_cast<int>(row))->continuation)};
        line.cells.assign(visible.cells.begin() + row * visible.size.columns,
                          visible.cells.begin() + (row + 1) * visible.size.columns);
        for (std::uint32_t column = visible.size.columns; column > 0; --column) {
            VTermScreenCell cell{};
            vterm_screen_get_cell(parser_state_->screen,
                                  {static_cast<int>(row), static_cast<int>(column - 1)}, &cell);
            if (cell.chars[0] != 0) {
                line.used_columns = column;
                break;
            }
        }
        captured.lines.push_back(std::move(line));
    }
    return captured;
}

std::string TerminalState::take_program_replies() {
    return std::exchange(parser_state_->replies, {});
}
} // namespace tmux_cxx::terminal

/** @brief Capture row boundaries before generated external scroll or resize code moves them. */
extern "C" void tmux_cxx_vterm_prepare_history_rows(VTerm *terminal, const VTermLineInfo *rows,
                                                    int count) noexcept {
    tmux_cxx::terminal::VTermHistoryCallbacks::prepare_rows(terminal, rows, count);
}

/** @brief Prepare one outgoing row's boundary before the existing external history callback. */
extern "C" void tmux_cxx_vterm_prepare_history_row(VTerm *terminal, int row) noexcept {
    tmux_cxx::terminal::VTermHistoryCallbacks::prepare_row(terminal, row);
}

/** @brief Restore the logical-line boundary belonging to one popped history row. */
extern "C" int tmux_cxx_vterm_popped_history_continuation(VTerm *terminal) noexcept {
    return tmux_cxx::terminal::VTermHistoryCallbacks::popped_continuation(terminal);
}
