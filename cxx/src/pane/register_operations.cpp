#include "register_operations.hpp"

#include <tmux_cxx/pane/operations/begin_copy_selection.hpp>
#include <tmux_cxx/pane/operations/cancel_copy_mode.hpp>
#include <tmux_cxx/pane/operations/clear_copy_selection.hpp>
#include <tmux_cxx/pane/operations/copy_selection.hpp>
#include <tmux_cxx/pane/operations/enter_copy_mode.hpp>
#include <tmux_cxx/pane/operations/list_panes.hpp>
#include <tmux_cxx/pane/operations/move_copy_cursor.hpp>
#include <tmux_cxx/pane/operations/move_copy_cursor_to.hpp>
#include <tmux_cxx/pane/operations/read_copy_mode.hpp>
#include <tmux_cxx/pane/operations/read_pane_output.hpp>
#include <tmux_cxx/pane/operations/read_pane_text.hpp>
#include <tmux_cxx/pane/operations/resize_pane.hpp>
#include <tmux_cxx/pane/operations/scroll_copy_mode.hpp>
#include <tmux_cxx/pane/operations/select_pane.hpp>
#include <tmux_cxx/pane/operations/send_pane_input.hpp>
#include <tmux_cxx/pane/operations/split_pane.hpp>
#include <tmux_cxx/pane/operations/wait_pane_output.hpp>

namespace tmux_cxx {
Result<void> register_pane_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<SendPaneInput>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ReadPaneOutput>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<WaitPaneOutput>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ReadPaneText>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SplitPane>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectPane>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ListPanes>(); !registration) {
        return registration;
    }
    if (auto added = registry.add<EnterCopyMode>(); !added) {
        return added;
    }
    if (auto added = registry.add<MoveCopyCursor>(); !added) {
        return added;
    }
    if (auto added = registry.add<MoveCopyCursorTo>(); !added) {
        return added;
    }
    if (auto added = registry.add<ScrollCopyMode>(); !added) {
        return added;
    }
    if (auto added = registry.add<BeginCopySelection>(); !added) {
        return added;
    }
    if (auto added = registry.add<ClearCopySelection>(); !added) {
        return added;
    }
    if (auto added = registry.add<CancelCopyMode>(); !added) {
        return added;
    }
    if (auto added = registry.add<CopySelection>(); !added) {
        return added;
    }
    if (auto added = registry.add<ReadCopyMode>(); !added) {
        return added;
    }
    return registry.add<ResizePane>();
}
} // namespace tmux_cxx
