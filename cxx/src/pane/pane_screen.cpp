#include "pane/pane.hpp"

#include <utility>

namespace tmux_cxx {
PaneScreenSnapshot Pane::screen_snapshot() const {
    if (copy_mode) {
        auto screen = copy_mode->screen_snapshot();
        screen.revision = display_revision();
        return screen;
    }
    auto base = terminal.snapshot();
    return {base.size,     base.cursor,        base.cursor_visible,
            base.complete, display_revision(), std::move(base.cells)};
}

void Pane::touch_copy_display() noexcept { ++copy_display_changes_; }

std::uint64_t Pane::display_revision() const noexcept {
    const auto source = copy_mode ? copy_mode->state().source_revision : terminal.revision();
    return source + copy_display_changes_;
}
} // namespace tmux_cxx
