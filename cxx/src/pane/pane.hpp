#pragma once

#include "pane/copy_mode.hpp"
#include "pane/pane_output_buffer.hpp"
#include "pane/pane_program_input.hpp"
#include "platform/posix/owned_fd.hpp"
#include "terminal/terminal_state.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/pane_screen_snapshot.hpp>

#include <optional>
#include <string>

namespace tmux_cxx {
/** @brief Own a pane's PTY and retained output independently of program termination and reaping. */
struct Pane {
    /** @brief Bind a newly allocated pane to one owning window and initial terminal geometry. */
    Pane(PaneId identity, WindowId owner, terminal::CellSize size)
        : id(identity), window(owner), terminal(size) {}
    /** @brief Advance queued input and available output within separate 64-KiB byte budgets. */
    void advance_pty_io();
    /** @brief Admit user bytes after earlier terminal replies; reject insufficient queue capacity
     * without admitting user bytes. */
    Result<void> admit_program_input(std::string_view bytes);
    /** @brief Deliver admitted input and terminal replies while consuming the shared remaining
     * byte budget. */
    Result<void> flush_program_input(std::size_t &remaining_bytes);
    /** @brief Return safe admission bytes, or no value once program input permanently closes. */
    std::optional<std::size_t> client_program_input_capacity() const;
    /** @brief Resize retained terminal state and notify the live PTY program. */
    Result<void> resize_terminal(terminal::CellSize size);
    /** @brief Copy the pane's displayed cells and cursor without exposing parser or program facts.
     */
    PaneScreenSnapshot screen_snapshot() const;
    /** @brief Advance pane display freshness after a committed copy-mode change. */
    void touch_copy_display() noexcept;
    /** @brief Combine the selected source revision with committed copy-display changes. */
    std::uint64_t display_revision() const noexcept;
    PaneId id;               ///< Stable server-local pane identity.
    WindowId window;         ///< Shared window that exclusively owns this pane.
    posix::OwnedFd pty;      ///< Owned PTY master retained through output draining.
    PaneOutputBuffer output; ///< Bounded raw PTY stream with absolute observation cursors.
    terminal::TerminalState
        terminal; ///< Retained parser, screen, and history independent of raw retention.
    std::optional<CopyMode>
        copy_mode;                  ///< Pane-owned frozen display retained through client disposal.
    PaneProgramInput program_input; ///< Ordered input awaiting nonblocking PTY delivery.
    std::string terminal_replies;   ///< Earlier query replies awaiting input capacity before later
                                    ///< output is parsed.

  private:
    std::uint64_t copy_display_changes_ =
        0; ///< Committed mode changes accumulated across entry and exit.
};
} // namespace tmux_cxx
