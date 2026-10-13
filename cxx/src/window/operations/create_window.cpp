#include <tmux_cxx/window/operations/create_window.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "text/utf8.hpp"

#include <algorithm>
#include <limits>

namespace tmux_cxx {
Result<WindowLinkSnapshot> Server::create_window(SessionId session_id, std::uint32_t index,
                                                 std::string name, std::string command) {
    const auto session = state_->sessions.find(session_id);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    if (index > static_cast<std::uint32_t>(std::numeric_limits<int>::max()) || name.empty() ||
        name.size() > 128 || !text::valid_utf8(name) ||
        std::ranges::any_of(name, /** @brief Reject control bytes in explicit window names. */
                            [](unsigned char byte) { return byte < 0x20 || byte == 0x7f; }) ||
        command.empty() || command.find('\0') != command.npos) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid window index, name, or command"});
    }
    if (session->second.windows.contains(index)) {
        return std::unexpected(TmuxError{TmuxErrorCode::duplicate_index,
                                         "window index already exists in this session"});
    }
    const auto size =
        state_->windows.at(state_->window_links.at(session->second.current).window).size;
    const WindowId window_id{state_->next_window_identity++};
    const WindowLinkId link_id{state_->next_window_link_identity++};
    const PaneId pane_id{state_->next_pane_identity++};
    if (auto spawned = spawn_pane(pane_id, window_id, size, command); !spawned) {
        return std::unexpected(spawned.error());
    }
    const auto revision = ++state_->revision;
    state_->windows.emplace(window_id, Window{window_id,
                                              std::move(name),
                                              WindowLayout{pane_id, size},
                                              pane_id,
                                              {link_id},
                                              size,
                                              WindowSizePolicy::latest,
                                              revision});
    const auto link =
        state_->window_links
            .emplace(link_id, WindowLink{link_id, session_id, window_id, index, revision})
            .first;
    session->second.windows.emplace(index, link_id);
    session->second.revision = revision;
    return state_->window_link_snapshot(link->second);
}

CreateWindow::Request CreateWindow::read(protocol::native::NativePayloadReader &reader) {
    return {SessionId{reader.u64()}, reader.u32(), reader.string(), reader.string()};
}
Result<CreateWindow::Response> CreateWindow::execute(Server &server, Request request) {
    return server.create_window(request.session, request.index, std::move(request.name),
                                std::move(request.command));
}
protocol::native::OperationReply CreateWindow::write(Response response) {
    protocol::native::NativePayloadWriter reply;
    reply.window_link(response);
    return std::move(reply.bytes);
}
} // namespace tmux_cxx
