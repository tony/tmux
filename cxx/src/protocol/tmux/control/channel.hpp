#pragma once

#include "platform/posix/owned_fd.hpp"
#include "protocol/tmux/control/command_result.hpp"

#include <tmux_cxx/tmux_error.hpp>

#include <memory>
#include <string>
#include <string_view>
#include <vector>

namespace tmux_cxx::protocol::tmux::control {
/** @brief Carry complete command lines and an orderly exit request from one ready read. */
struct InputBatch {
    std::vector<std::string> command_lines; ///< Complete nonempty lines in arrival order.
    bool exit_requested; ///< Empty line or end of input ends this control-client lifetime.
};

/** @brief Own bounded descriptor I/O for one accepted stock control-mode channel. */
class Channel {
  public:
    /** @brief Adopt distinct control input and output routes and make them nonblocking. */
    static Result<std::unique_ptr<Channel>> adopt(posix::OwnedFd input, posix::OwnedFd output);
    /** @brief Restore shared descriptor status flags before releasing the control routes. */
    ~Channel();
    /** @brief Prevent copying descriptor and queue ownership. */
    Channel(const Channel &) = delete;
    /** @brief Prevent assigning shared descriptor and queue ownership. */
    Channel &operator=(const Channel &) = delete;
    /** @brief Queue one complete success or error block under the selected stock grammar. */
    Result<void> queue_command_result(const TmuxControlModeProfile &grammar, CommandGuard guard,
                                      CommandOutcome outcome, std::string_view body);
    /** @brief Queue one complete newline-terminated control record after previously accepted bytes.
     */
    Result<void> queue_record(std::string record);
    /** @brief Read complete newline commands within a byte budget without waiting. */
    Result<InputBatch> read_input(std::size_t byte_budget);
    /** @brief Deliver queued control bytes through partial writes within a byte budget. */
    Result<void> flush_output(std::size_t &byte_budget);
    /** @brief Report whether control records still await descriptor delivery. */
    bool output_pending() const;
    /** @brief Borrow the control input descriptor until channel destruction. */
    int input_descriptor() const;
    /** @brief Borrow the control output descriptor until channel destruction. */
    int output_descriptor() const;

  private:
    /** @brief Retain adopted routes and their original shared status flags. */
    Channel(posix::OwnedFd input, posix::OwnedFd output, int input_flags, int output_flags);
    /** @brief Append complete bytes within the shared bounded control-output queue. */
    Result<void> queue_output_bytes(std::string bytes);
    posix::OwnedFd input_;        ///< Passed stock-client standard input route.
    posix::OwnedFd output_;       ///< Passed stock-client standard output route.
    int input_flags_;             ///< Shared input status flags restored before socket exit.
    int output_flags_;            ///< Shared output status flags restored before socket exit.
    std::string input_bytes_;     ///< Incomplete control line retained across ready reads.
    std::string output_bytes_;    ///< Guarded blocks awaiting nonblocking delivery.
    std::size_t output_sent_ = 0; ///< Delivered prefix retained until queue compaction.
};
} // namespace tmux_cxx::protocol::tmux::control
