#pragma once

#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <memory>
#include <string_view>

namespace tmux_cxx::terminal {
/** @brief Own retained VT parser, screen, and bounded history; callers serialize all access. */
class TerminalState {
  public:
    /** @brief Create at most 16384 cells, 512 rows/columns, and 1000 history lines; invalid
     * dimensions throw invalid_argument. */
    explicit TerminalState(CellSize size = {24, 80}, std::size_t history_limit = 1000);
    /** @brief Release parser, screens, and terminal history without controlling a program or
     * client. */
    ~TerminalState();
    /** @brief Prevent copying a live parser and its continuation state. */
    TerminalState(const TerminalState &) = delete;
    /** @brief Prevent replacing terminal state through copy assignment. */
    TerminalState &operator=(const TerminalState &) = delete;
    /** @brief Consume output in stream order; contained callback failure reports incomplete state
     * and prevents further mutation. */
    Result<void> ingest_program_output(std::string_view bytes);
    /** @brief Resize within constructor bounds; invalid dimensions preserve state and callback
     * failures report incomplete state. */
    Result<void> resize(CellSize size);
    /** @brief Copy visible cells and cursor state; retained terminal history cannot recover
     * discarded raw output. */
    TerminalSnapshot snapshot() const;
    /** @brief Transfer ordered terminal replies, bounded to 64 KiB, for delivery to the program. */
    std::string take_program_replies();

  private:
    struct ParserState;
    std::unique_ptr<ParserState>
        parser_state_; ///< Exclusive parser, retained screen, and callback-state ownership.
};
} // namespace tmux_cxx::terminal
