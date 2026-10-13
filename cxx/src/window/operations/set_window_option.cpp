#include <tmux_cxx/window/operations/set_window_option.hpp>

#include "server/state.hpp"
#include "window/window_option_target.hpp"
#include "window/window_options.hpp"

#include <utility>
#include <vector>

namespace tmux_cxx {
Result<void> Server::set_window_option(WindowOptionTarget target, std::string_view name,
                                       std::string_view value) {
    if (const auto *window_id = std::get_if<WindowId>(&target)) {
        if (window_id->value == 0) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "window identity must be nonzero"});
        }
        const auto window = state_->windows.find(*window_id);
        if (window == state_->windows.end()) {
            return std::unexpected(TmuxError{TmuxErrorCode::not_found, "window no longer exists"});
        }
        const auto parsed = window_options::catalog().parse(name, value);
        if (!parsed) {
            return std::unexpected(parsed.error());
        }
        const auto changed = window->second.options.set(*parsed);
        if (!changed) {
            return std::unexpected(changed.error());
        }
        if (*changed) {
            window->second.revision = ++state_->revision;
        }
        return {};
    }

    const auto parsed = window_options::catalog().parse(name, value);
    if (!parsed) {
        return std::unexpected(parsed.error());
    }
    auto next_defaults = state_->global_window_options;
    const auto changed = next_defaults.set(*parsed);
    if (!changed) {
        return std::unexpected(changed.error());
    }
    if (!*changed) {
        return {};
    }
    std::vector<WindowId> changed_windows;
    changed_windows.reserve(state_->windows.size());
    for (const auto &[window_id, window] : state_->windows) {
        const auto previous =
            window.options.effective(*parsed->definition, &state_->global_window_options);
        const auto next = window.options.effective(*parsed->definition, &next_defaults);
        if (!previous || !next) {
            return std::unexpected(!previous ? previous.error() : next.error());
        }
        if (*previous != *next) {
            changed_windows.push_back(window_id);
        }
    }
    // Candidate defaults and affected-window storage are complete before changing live state.
    state_->global_window_options = std::move(next_defaults);
    const auto revision = ++state_->revision;
    for (const auto window_id : changed_windows) {
        state_->windows.at(window_id).revision = revision;
    }
    return {};
}

SetWindowOption::Request SetWindowOption::read(protocol::native::NativePayloadReader &reader) {
    return {read_window_option_target(reader), reader.string(), reader.string()};
}

Result<SetWindowOption::Response> SetWindowOption::execute(Server &server, Request request) {
    return server.set_window_option(request.target, request.name, request.value);
}

protocol::native::OperationReply SetWindowOption::write() { return protocol::native::Bytes{}; }
} // namespace tmux_cxx
