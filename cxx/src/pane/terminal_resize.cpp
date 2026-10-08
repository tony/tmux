#include "pane/pane.hpp"

#include "platform/posix/errno_error.hpp"

#include <sys/ioctl.h>

namespace tmux_cxx {
Result<void> Pane::resize_terminal(terminal::CellSize size) {
    if (pty.get() >= 0) {
        const winsize pty_size{size.rows, size.columns, 0, 0};
        if (::ioctl(pty.get(), TIOCSWINSZ, &pty_size) < 0) {
            return std::unexpected(posix::errno_error("pane program terminal size"));
        }
    }
    return terminal.resize(size);
}
} // namespace tmux_cxx
