#pragma once

#include "pane/pane_output_buffer.hpp"
#include "pane/pane_program_input.hpp"
#include "platform/posix/owned_fd.hpp"
#include "terminal/terminal_state.hpp"

#include <tmux_cxx/identity.hpp>

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
    /** @brief Return bytes safely readable from a client before all could enter the input queue. */
    std::size_t client_program_input_capacity() const;
    /** @brief Resize retained terminal state and notify the live PTY program. */
    Result<void> resize_terminal(terminal::CellSize size);
    PaneId id;               ///< Stable server-local pane identity.
    WindowId window;         ///< Shared window that exclusively owns this pane.
    posix::OwnedFd pty;      ///< Owned PTY master retained through output draining.
    PaneOutputBuffer output; ///< Bounded raw PTY stream with absolute observation cursors.
    terminal::TerminalState
        terminal; ///< Retained parser, screen, and history independent of raw retention.
    PaneProgramInput program_input; ///< Ordered input awaiting nonblocking PTY delivery.
    std::string terminal_replies;   ///< Earlier query replies awaiting input capacity before later
                                    ///< output is parsed.
};
} // namespace tmux_cxx
