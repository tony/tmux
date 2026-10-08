#pragma once

#include "platform/posix/owned_fd.hpp"
#include <tmux_cxx/tmux_error.hpp>

#include <chrono>
#include <optional>
#include <sys/types.h>

namespace tmux_cxx {
/** @brief Retain direct-child ownership after pane removal and reap only that child without
 * blocking. */
class PaneProgram {
  public:
    using Clock = std::chrono::steady_clock; ///< Monotonic termination deadlines.
    /** @brief Prepare an unlaunched program record before allocating a child process. */
    PaneProgram() = default;
    /** @brief Force any remaining child to stop and attempt a final nonblocking reap. */
    ~PaneProgram();
    /** @brief Prevent duplicating ownership of a direct child and its pidfd. */
    PaneProgram(const PaneProgram &) = delete;
    /** @brief Prevent replacing the owner of a direct child. */
    PaneProgram &operator=(const PaneProgram &) = delete;
    /** @brief Adopt the forked child and establish pidfd readiness; failures retain ownership for
     * cleanup. */
    Result<void> adopt(pid_t child_pid);
    /** @brief Send HUP once and allow 250 milliseconds before forced termination. */
    void stop();
    /** @brief Reap or relinquish a collected child and escalate an expired stop deadline without
     * waiting. */
    void advance_reaping(Clock::time_point now);
    /** @brief Borrow child-exit readiness until the next reaping turn. */
    int readiness_descriptor() const { return readiness_.get(); }
    /** @brief Report whether the direct child has been reaped, independently of PTY EOF. */
    bool reaped() const { return child_pid_ < 0; }
    /** @brief Borrow a wait status collected here; external reaping leaves the status unknown. */
    const std::optional<int> &wait_status() const { return wait_status_; }
    /** @brief Return the pending HUP escalation deadline, if any. */
    const std::optional<Clock::time_point> &termination_deadline() const {
        return termination_deadline_;
    }

  private:
    /** @brief Distinguish admission, graceful termination, forced termination, and completed child
     * collection. */
    enum class Phase { unlaunched, running, hangup_sent, kill_sent, reaped };
    /** @brief Signal the owned process group and use pidfd for its direct child when available. */
    void signal_process_group(int signal_number);
    pid_t child_pid_ = -1; ///< Owned child; cleared after collection or loss of wait ownership.
    Phase phase_ =
        Phase::unlaunched; ///< Program lifecycle independent of pane retention and PTY EOF.
    posix::OwnedFd
        readiness_; ///< Owned pidfd, preventing readiness from referring to a reused PID.
    std::optional<int> wait_status_; ///< Raw wait status retained independently of pane output.
    std::optional<Clock::time_point>
        termination_deadline_; ///< HUP escalation deadline, cleared after KILL or reaping.
};
} // namespace tmux_cxx
