#include <tmux_cxx/pane/operations/copy_selection.hpp>
#include <tmux_cxx/paste_buffer/operations/add_paste_buffer.hpp>

#include "pane/copy_mode_target.hpp"
#include "paste_buffer/paste_buffer.hpp"
#include "server/state.hpp"

namespace tmux_cxx {
Result<std::optional<PasteBufferSnapshot>>
Server::copy_selection(PaneId pane_id, CopySelectionDisposition disposition) {
    if (!valid_copy_selection_disposition(disposition)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid copy disposition"});
    }
    auto target = copy_mode_pane(state_->panes, pane_id);
    if (!target) {
        return std::unexpected(target.error());
    }
    auto &pane = **target;
    auto mode = active_copy_mode(pane);
    if (!mode) {
        return std::unexpected(mode.error());
    }
    auto keys = window_mode_keys(pane.window);
    if (!keys) {
        return std::unexpected(keys.error());
    }
    auto bytes = (*mode)->selected_text(*keys, paste_buffer_byte_limit);
    if (!bytes) {
        return std::unexpected(bytes.error());
    }
    auto stored = AddPasteBuffer::execute(*this, {std::move(*bytes)});
    if (!stored) {
        return std::unexpected(stored.error());
    }
    if (disposition == CopySelectionDisposition::cancel_mode) {
        pane.copy_mode.reset();
        pane.touch_copy_display();
    } else if (disposition == CopySelectionDisposition::clear_selection) {
        auto changed = (*mode)->clear_selection(*keys);
        if (!changed) {
            return std::unexpected(changed.error());
        }
        if (*changed) {
            pane.touch_copy_display();
        }
    }
    return stored;
}
CopySelection::Request CopySelection::read(protocol::native::NativePayloadReader &reader) {
    return {PaneId{reader.u64()}, read_copy_mode_choice(reader, &valid_copy_selection_disposition)};
}
Result<CopySelection::Response> CopySelection::execute(Server &server, Request request) {
    return server.copy_selection(request.pane, request.disposition);
}
protocol::native::OperationReply CopySelection::write(Response response) {
    return AddPasteBuffer::write(std::move(response));
}
} // namespace tmux_cxx
