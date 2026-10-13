#include "pane/pane.hpp"

#include <utility>

namespace tmux_cxx {
PaneScreenSnapshot Pane::screen_snapshot() const {
    auto base = terminal.snapshot();
    return {base.size,     base.cursor,   base.cursor_visible,
            base.complete, base.revision, std::move(base.cells)};
}
} // namespace tmux_cxx
