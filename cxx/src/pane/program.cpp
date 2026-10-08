#include "pane/program.hpp"
#include "platform/posix/errno_error.hpp"

#include <cerrno>
#include <csignal>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

namespace tmux_cxx {
PaneProgram::~PaneProgram() {
    advance_reaping(Clock::now());
    if (!reaped()) {
        signal_process_group(SIGKILL);
        phase_ = Phase::kill_sent;
        advance_reaping(Clock::now());
    }
}
Result<void> PaneProgram::adopt(pid_t child_pid) {
    child_pid_ = child_pid;
    phase_ = Phase::running;
    readiness_.reset(static_cast<int>(::syscall(SYS_pidfd_open, child_pid, 0)));
    if (readiness_.get() < 0) {
        return std::unexpected(posix::errno_error("pidfd_open"));
    }
    return {};
}
void PaneProgram::signal_process_group(int signal_number) {
    if (child_pid_ > 0) {
        ::kill(-child_pid_, signal_number);
        if (readiness_.get() >= 0) {
            ::syscall(SYS_pidfd_send_signal, readiness_.get(), signal_number, nullptr, 0);
        } else {
            ::kill(child_pid_, signal_number);
        }
    }
}
void PaneProgram::stop() {
    const auto now = Clock::now();
    advance_reaping(now);
    if (phase_ == Phase::running) {
        signal_process_group(SIGHUP);
        phase_ = Phase::hangup_sent;
        termination_deadline_ = now + std::chrono::milliseconds(250);
    }
}
void PaneProgram::advance_reaping(Clock::time_point now) {
    if (child_pid_ < 0) {
        return;
    }
    int child_status = 0;
    pid_t waited_child;
    do {
        waited_child = ::waitpid(child_pid_, &child_status, WNOHANG);
    } while (waited_child < 0 && errno == EINTR);
    if (waited_child == child_pid_ || (waited_child < 0 && errno == ECHILD)) {
        if (waited_child == child_pid_) {
            wait_status_ = child_status;
        }
        child_pid_ = -1;
        phase_ = Phase::reaped;
        readiness_.reset();
        termination_deadline_.reset();
    } else if (termination_deadline_ && now >= *termination_deadline_) {
        signal_process_group(SIGKILL);
        phase_ = Phase::kill_sent;
        termination_deadline_.reset();
    }
}
} // namespace tmux_cxx
