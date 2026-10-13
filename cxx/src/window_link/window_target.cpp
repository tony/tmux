#include "window_link/window_target.hpp"

#include "commands/session_target.hpp"

#include <algorithm>
#include <charconv>
#include <limits>

namespace tmux_cxx {
Result<SessionWindowTarget> parse_session_window_target(std::string_view target) {
    const auto separator = target.find(':');
    const auto session = target.substr(0, separator);
    if (session.empty()) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window target requires an explicit session"});
    }
    SessionWindowTarget parsed{std::string(session), std::nullopt};
    if (separator == target.npos) {
        return parsed;
    }
    const auto index_text = target.substr(separator + 1);
    std::uint32_t index = 0;
    const auto [end, error] =
        std::from_chars(index_text.data(), index_text.data() + index_text.size(), index);
    if (error != std::errc{} || end != index_text.data() + index_text.size() ||
        index > static_cast<std::uint32_t>(std::numeric_limits<int>::max())) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window target requires an index in 0..2147483647"});
    }
    parsed.index = index;
    return parsed;
}
Result<WindowLinkSnapshot> resolve_window_link_target(const Server &server,
                                                      const SessionWindowTarget &target) {
    if (!target.index) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "window selection requires an explicit index"});
    }
    const auto session = commands::resolve_session_target(server, target.session);
    if (!session) {
        return std::unexpected(session.error());
    }
    const auto links = server.window_links(*session);
    if (!links) {
        return std::unexpected(links.error());
    }
    const auto link = std::ranges::find(*links, *target.index, &WindowLinkSnapshot::index);
    if (link == links->end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "window index no longer exists in this session"});
    }
    return *link;
}
} // namespace tmux_cxx
