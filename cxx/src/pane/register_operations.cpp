#include "register_operations.hpp"

#include <tmux_cxx/pane/operations/list_panes.hpp>
#include <tmux_cxx/pane/operations/read_pane_output.hpp>
#include <tmux_cxx/pane/operations/read_pane_text.hpp>
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
    return registry.add<ListPanes>();
}
} // namespace tmux_cxx
