#include "new_window_command.hpp"

#include "commands/session_target.hpp"

namespace tmux_cxx {
namespace {
/** @brief Report the exact implemented new-window grammar. */
TmuxError new_window_syntax_error() {
    return {TmuxErrorCode::invalid_argument,
            "supported syntax: new-window [-d] -t session[:index] -n name [command]"};
}
} // namespace
Result<NewWindowCommand::Request> NewWindowCommand::read(std::span<const std::string> arguments) {
    std::optional<SessionWindowTarget> target;
    std::optional<std::string> name;
    std::optional<std::string> command;
    bool detached = false;
    for (std::size_t index = 0; index < arguments.size(); ++index) {
        const auto &argument = arguments[index];
        if (argument == "-d") {
            if (detached) {
                return std::unexpected(new_window_syntax_error());
            }
            detached = true;
        } else if (argument == "-t") {
            if (target || ++index == arguments.size()) {
                return std::unexpected(new_window_syntax_error());
            }
            auto parsed = parse_session_window_target(arguments[index]);
            if (!parsed) {
                return std::unexpected(parsed.error());
            }
            target = std::move(*parsed);
        } else if (argument == "-n") {
            if (name || ++index == arguments.size()) {
                return std::unexpected(new_window_syntax_error());
            }
            name = arguments[index];
            if (name->find("#{") != name->npos) {
                return std::unexpected(new_window_syntax_error());
            }
        } else {
            if (argument.starts_with('-') || command || index + 1 != arguments.size()) {
                return std::unexpected(new_window_syntax_error());
            }
            command = argument;
        }
    }
    if (!target || !name) {
        return std::unexpected(new_window_syntax_error());
    }
    return Request{std::move(*target), std::move(*name), command.value_or("/bin/sh"), detached};
}
Result<commands::CommandOutcome> NewWindowCommand::execute(commands::CommandInvocation &invocation,
                                                           Request request) {
    const auto session =
        commands::resolve_session_target(invocation.server, request.target.session);
    if (!session) {
        return std::unexpected(session.error());
    }
    auto index = request.target.index;
    if (!index) {
        const auto links = invocation.server.window_links(*session);
        if (!links) {
            return std::unexpected(links.error());
        }
        index = 0;
        for (const auto &link : *links) {
            if (link.index != *index) {
                break;
            }
            ++*index;
        }
    }
    const auto created = CreateWindow::execute(
        invocation.server, {*session, *index, std::move(request.name), std::move(request.command)});
    if (!created) {
        return std::unexpected(created.error());
    }
    if (!request.detached) {
        if (auto selected = SelectWindow::execute(invocation.server, {created->id}); !selected) {
            return std::unexpected(selected.error());
        }
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
