#include "protocol/tmux/control/session_notification_feed.hpp"

#include "protocol/tmux/control/session_renamed.hpp"
#include "protocol/tmux/control/sessions_changed.hpp"
#include "session/session_event_reader.hpp"

#include <variant>

namespace tmux_cxx::protocol::tmux::control {
namespace {
/** @brief Exhaustively translate retained session events into stock control records. */
struct SessionNotificationFormatter {
    /** @brief Announce that the observable session catalog gained one entry. */
    std::string operator()(const SessionCreatedEvent &) const { return format_sessions_changed(); }
    /** @brief Preserve the renamed session identity and validated replacement name. */
    std::string operator()(const SessionRenamedEvent &event) const {
        return format_session_renamed(event);
    }
    /** @brief Announce that the observable session catalog lost one entry. */
    std::string operator()(const SessionClosedEvent &) const { return format_sessions_changed(); }
};
} // namespace

Result<void> SessionNotificationFeed::queue_notifications(const Server &server, Channel &channel) {
    auto event_batch = SessionEventReader::read(server, event_cursor_);
    if (!event_batch) {
        return std::unexpected(event_batch.error());
    }
    for (const auto &record : event_batch->records) {
        if (auto notification_queued =
                channel.queue_record(std::visit(SessionNotificationFormatter{}, record.event));
            !notification_queued) {
            return notification_queued;
        }
    }
    event_cursor_ = std::move(event_batch->next_cursor);
    return {};
}
} // namespace tmux_cxx::protocol::tmux::control
