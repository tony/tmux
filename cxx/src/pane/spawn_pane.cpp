#include <tmux_cxx/server/server.hpp>

#include "platform/posix/errno_error.hpp"
#include "server/state.hpp"

#include <csignal>
#include <cstdlib>
#include <fcntl.h>
#include <pty.h>
#include <sys/prctl.h>
#include <unistd.h>

namespace tmux_cxx {
Result<void> Server::spawn_pane(PaneId pane_id, WindowId window_id, terminal::CellSize size,
                                std::string_view command) {
    if (state_->shutdown_attempted) {
        return std::unexpected(TmuxError{TmuxErrorCode::closed, "server is shutting down"});
    }
    if (command.empty() || command.find('\0') != command.npos) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument, "invalid pane command"});
    }
    std::string shell_command(command);
    if (pane_id.value == 0 || window_id.value == 0 || state_->panes.contains(pane_id) ||
        state_->pane_programs.contains(pane_id)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid or duplicate pane identity"});
    }
    auto &pane = *state_->panes.emplace(pane_id, std::make_unique<Pane>(pane_id, window_id, size))
                      .first->second;
    auto &pane_program = state_->pane_programs.try_emplace(pane_id).first->second;
    int pty_descriptor = -1;
    winsize initial_pty_size{size.rows, size.columns, 0, 0};
    const auto server_process_id = ::getpid();
    const auto child_process_id = ::forkpty(&pty_descriptor, nullptr, nullptr, &initial_pty_size);
    if (child_process_id < 0) {
        const auto spawn_error = posix::errno_error("forkpty");
        state_->retire_pane(pane_id);
        return std::unexpected(spawn_error);
    }
    if (child_process_id == 0) {
        if (::prctl(PR_SET_PDEATHSIG, SIGKILL) < 0 || ::getppid() != server_process_id) {
            ::_exit(127);
        }
        sigset_t child_signal_mask;
        ::sigemptyset(&child_signal_mask);
        ::sigprocmask(SIG_SETMASK, &child_signal_mask, nullptr);
        for (int signal : {SIGTERM, SIGINT, SIGHUP, SIGCHLD, SIGPIPE}) {
            ::signal(signal, SIG_DFL);
        }
        ::unsetenv("TMUX");
        ::unsetenv("TMUX_PANE");
        ::setenv("TERM", "screen-256color", 1);
        ::execl("/bin/sh", "sh", "-c", shell_command.c_str(), static_cast<char *>(nullptr));
        ::_exit(127);
    }
    pane.pty.reset(pty_descriptor);
    auto child_adoption = pane_program.adopt(child_process_id);
    if (!child_adoption) {
        state_->retire_pane(pane_id);
        return std::unexpected(child_adoption.error());
    }
    if (::fcntl(pty_descriptor, F_SETFD, FD_CLOEXEC) < 0 ||
        ::fcntl(pty_descriptor, F_SETFL, O_NONBLOCK) < 0) {
        const auto flag_error = posix::errno_error("PTY flags");
        state_->retire_pane(pane_id);
        return std::unexpected(flag_error);
    }
    return {};
}
} // namespace tmux_cxx
