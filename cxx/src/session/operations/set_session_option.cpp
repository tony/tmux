#include <tmux_cxx/session/operations/set_session_option.hpp>

#include "protocol/native/framing.hpp"
#include "server/state.hpp"
#include "session/session_options.hpp"

#include <utility>

namespace tmux_cxx {
namespace {
/** @brief Parse and assign one session-scoped option, reporting whether its table changed. */
Result<bool> assign_session_option(options::OptionTable &table, std::string_view name,
                                   std::string_view value) {
    auto parsed_option = session_options::catalog().parse(name, value);
    if (!parsed_option) {
        return std::unexpected(parsed_option.error());
    }
    return table.set(*parsed_option);
}
} // namespace

Result<void> Server::set_global_session_option(std::string_view name, std::string_view value) {
    auto option_changed = assign_session_option(state_->global_session_options, name, value);
    if (!option_changed) {
        return std::unexpected(option_changed.error());
    }
    if (*option_changed) {
        ++state_->revision;
    }
    return {};
}

Result<void> Server::set_session_option(SessionId session_id, std::string_view name,
                                        std::string_view value) {
    auto session = state_->sessions.find(session_id);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    auto option_changed = assign_session_option(session->second.options, name, value);
    if (!option_changed) {
        return std::unexpected(option_changed.error());
    }
    if (*option_changed) {
        session->second.revision = ++state_->revision;
    }
    return {};
}

Result<SessionPrefixKeys> Server::session_prefix_keys(SessionId session_id) const {
    const auto session = state_->sessions.find(session_id);
    if (session == state_->sessions.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "session no longer exists"});
    }
    return session_options::prefix_keys(session->second.options, state_->global_session_options);
}

SetSessionOption::Request SetSessionOption::read(protocol::native::NativePayloadReader &reader) {
    auto name = reader.string();
    auto value = reader.string();
    SessionOptionTarget target = GlobalSessionOptions{};
    if (!reader.finished()) {
        target = SessionId{reader.u64()};
    }
    return {target, std::move(name), std::move(value)};
}

Result<SetSessionOption::Response> SetSessionOption::execute(Server &server, Request request) {
    if (const auto *session = std::get_if<SessionId>(&request.target)) {
        return server.set_session_option(*session, request.name, request.value);
    }
    return server.set_global_session_option(request.name, request.value);
}

protocol::native::OperationReply SetSessionOption::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
