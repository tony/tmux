#pragma once

#include <tmux_cxx/pane/pane_text_snapshot.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>

namespace tmux_cxx {
/** @brief Describe owner-qualified interpreted pane text without equating it with raw-output
 * retention. */
struct ReadPaneText {
    /** @brief Resolve one pane against the current server instance. */
    struct Request {
        PaneId pane; ///< Pane identity resolved against current server state.
    };
    using Response = PaneTextSnapshot; ///< Owned text projection independent of cell rendition and
                                       ///< client viewports.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{12}, "pane.read_text",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(protocol::native::Capability::pane_terminal_text)};
    ///< Stable identity, query effects, and required terminal-text capability.
    /** @brief Decode the pane identity before observing interpreted text. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::read_pane_text */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode copied text, terminal freshness, and program-input state within native framing
     * bounds. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
