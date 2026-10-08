#pragma once

#include "platform/posix/owned_fd.hpp"

#include <tmux_cxx/terminal/terminal_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <memory>
#include <string>
#include <termios.h>

namespace tmux_cxx {
/** @brief Carry one bounded terminal-input read and distinguish readiness from route closure. */
struct ClientTerminalInput {
    std::string bytes; ///< Input bytes read in order from the ordinary terminal.
    bool closed;       ///< The terminal input route reached end of file.
};

/** @brief Record whether usable client dimensions came from the TTY or the zero-size fallback. */
enum class ClientTerminalSizeOrigin { measured, zero_dimension_defaults };

/** @brief Own one ordinary client's terminal descriptor, saved mode, and measured cell size. */
class ClientTerminal {
  public:
    /** @brief Validate and adopt one terminal's input and output routes without changing its mode.
     */
    static Result<std::unique_ptr<ClientTerminal>> adopt(posix::OwnedFd input,
                                                         posix::OwnedFd output);
    /** @brief Restore an activated terminal before releasing its descriptor. */
    ~ClientTerminal();
    /** @brief Prevent copying descriptor and restoration ownership. */
    ClientTerminal(const ClientTerminal &) = delete;
    /** @brief Prevent replacing descriptor and restoration ownership. */
    ClientTerminal &operator=(const ClientTerminal &) = delete;
    /** @brief Enter raw mode once; failure leaves the client detached. */
    Result<void> activate();
    /** @brief Restore the saved terminal mode once when raw-mode ownership ends. */
    Result<void> restore();
    /** @brief Append one complete redraw or reject it before effects when output is full. */
    Result<void> queue_output(std::string view_bytes);
    /** @brief Deliver queued view bytes through partial nonblocking writes within a byte budget. */
    Result<void> flush_output(std::size_t &remaining_output_bytes);
    /** @brief Read at most limit input bytes without waiting; zero limit performs no read. */
    Result<ClientTerminalInput> read_input(std::size_t limit);
    /** @brief Report whether composed view bytes still await terminal delivery. */
    bool output_pending() const;
    /** @brief Report whether raw mode and the tmux view currently own this terminal. */
    bool active() const;
    /** @brief Return measured outer-terminal dimensions used by attachment and view composition. */
    terminal::CellSize size() const;
    /** @brief Report whether the retained dimensions were measured or substituted for zeros. */
    ClientTerminalSizeOrigin size_origin() const;
    /** @brief Remeasure the retained output TTY and update its bounded outer dimensions. */
    Result<terminal::CellSize> measure();
    /** @brief Borrow the terminal input descriptor until client destruction. */
    int input_descriptor() const;
    /** @brief Borrow the terminal output descriptor until client destruction. */
    int output_descriptor() const;

  private:
    /** @brief Retain validated routes, attributes, flags, and dimensions before attachment. */
    ClientTerminal(posix::OwnedFd input, posix::OwnedFd output, termios saved_mode, int input_flags,
                   int output_flags, terminal::CellSize size, ClientTerminalSizeOrigin size_origin);
    posix::OwnedFd input_;    ///< Owned ordinary-terminal input route.
    posix::OwnedFd output_;   ///< Owned ordinary-terminal output route.
    termios saved_mode_{};    ///< Attributes restored when the attachment route ends.
    int saved_input_flags_;   ///< Shared input status flags restored with terminal ownership.
    int saved_output_flags_;  ///< Shared output status flags restored with terminal ownership.
    terminal::CellSize size_; ///< Measured outer-terminal rows and columns.
    ClientTerminalSizeOrigin size_origin_; ///< Source of the retained nonzero dimensions.
    bool active_ = false;         ///< Raw mode and nonblocking flags still require restoration.
    std::string output_bytes_;    ///< Composed views awaiting nonblocking terminal delivery.
    std::size_t output_sent_ = 0; ///< Delivered prefix retained until queue compaction.
    bool view_active_ = false;    ///< The outer terminal may require alternate-screen teardown.
};
} // namespace tmux_cxx
