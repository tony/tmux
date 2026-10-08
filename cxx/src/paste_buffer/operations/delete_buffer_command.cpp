#include "delete_buffer_command.hpp"

namespace tmux_cxx {
Result<DeleteBufferCommand::Request>
DeleteBufferCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 2 || arguments[0] != "-b") {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "supported syntax: delete-buffer -b name"});
    }
    return Request{arguments[1]};
}

Result<commands::CommandOutcome>
DeleteBufferCommand::execute(commands::CommandInvocation &invocation, Request request) {
    if (auto deleted = DeletePasteBuffer::execute(invocation.server, std::move(request));
        !deleted) {
        return std::unexpected(deleted.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
