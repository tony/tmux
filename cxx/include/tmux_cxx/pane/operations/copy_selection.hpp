#pragma once

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/pane/copy_selection_disposition.hpp>
#include <tmux_cxx/paste_buffer/paste_buffer_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Store selected cells in an automatic buffer before applying the requested mode
 * disposition. */
struct CopySelection {
    /** @brief Own a pane target and validated copy-mode arguments until server execution. */
    struct Request {
        PaneId pane; ///< Pane whose retained mode is addressed.
        CopySelectionDisposition
            disposition; ///< Selection or mode change after successful storage.
    };
    using Response = std::optional<PasteBufferSnapshot>; ///< Created automatic-buffer metadata,
                                                         ///< absent for empty extraction.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{47}, "pane.copy_selection",
        protocol::native::OperationEffect::mutation,
        protocol::native::capability_mask(protocol::native::Capability::copy_mode)};
    ///< Stable copy-mode identity and required native behavior.
    /** @brief Decode the pane and closed copy arguments before server effects. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::copy_selection */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode optional automatic-buffer metadata without copying selected bytes again. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
