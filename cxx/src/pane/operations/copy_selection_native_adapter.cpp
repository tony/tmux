#include "connection/native_server_channel.hpp"
#include "paste_buffer/paste_buffer_native_codec.hpp"
#include <tmux_cxx/pane/operations/copy_selection.hpp>

#include <stdexcept>

namespace tmux_cxx {
Result<std::optional<PasteBufferSnapshot>>
ServerConnection::copy_selection(PaneId pane, CopySelectionDisposition disposition) {
    if (pane.value == 0 || !valid_copy_selection_disposition(disposition)) {
        return std::unexpected(channel_->operation_error(CopySelection::description,
                                                         TmuxErrorCode::invalid_argument,
                                                         "invalid copy-mode target or choice"));
    }
    protocol::native::NativePayloadWriter arguments;
    arguments.u64(pane.value);
    arguments.u32(std::to_underlying(disposition));
    auto reply = channel_->invoke_operation(CopySelection::description, std::move(arguments));
    if (!reply) {
        return std::unexpected(reply.error());
    }
    try {
        protocol::native::NativePayloadReader reader(*reply);
        const auto present = reader.u16();
        if (present > 1) {
            throw std::invalid_argument("invalid copied-buffer optional flag");
        }
        std::optional<PasteBufferSnapshot> snapshot;
        if (present != 0) {
            snapshot = protocol::native::read_paste_buffer_snapshot(reader);
            if (!snapshot->automatic) {
                throw std::invalid_argument("copy-selection returned a named buffer");
            }
        }
        reader.require_end();
        return snapshot;
    } catch (const std::invalid_argument &error) {
        return std::unexpected(channel_->operation_error(CopySelection::description,
                                                         TmuxErrorCode::protocol, error.what()));
    } catch (const std::length_error &error) {
        return std::unexpected(channel_->operation_error(CopySelection::description,
                                                         TmuxErrorCode::protocol, error.what()));
    }
}
} // namespace tmux_cxx
