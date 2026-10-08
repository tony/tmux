#pragma once

#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/window_link/window_link_snapshot.hpp>

namespace tmux_cxx {
/** @brief Declare observation of the indexed window memberships belonging to one session. */
struct ListWindowLinks {
    /** @brief Observe memberships of a session resolved at execution time. */
    struct Request {
        SessionId session; ///< Session whose memberships are observed.
    };
    using Response =
        std::vector<WindowLinkSnapshot>; ///< Materialized memberships ordered by session index.
    static constexpr protocol::native::OperationDescription description{
        protocol::native::OperationId{10}, "window.list_links",
        protocol::native::OperationEffect::query,
        protocol::native::capability_mask(
            protocol::native::Capability::window_links)}; ///< Stable native identity and effects.
    /** @brief Decode the owning session identity before observing memberships. */
    static Request read(protocol::native::NativePayloadReader &reader);
    /** @copybrief tmux_cxx::Server::window_links */
    static Result<Response> execute(Server &server, Request request);
    /** @brief Encode ordered membership values as a bounded native list. */
    static protocol::native::OperationReply write(Response response);
};
} // namespace tmux_cxx
