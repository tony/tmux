#include "set_buffer_command.hpp"

namespace tmux_cxx {
Result<SetBufferCommand::Request> SetBufferCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() != 3 || arguments[0] != "-b") {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "supported syntax: set-buffer -b name data"});
    }
    return Request{arguments[1], arguments[2]};
}

Result<commands::CommandOutcome> SetBufferCommand::execute(commands::CommandInvocation &invocation,
                                                           Request request) {
    if (auto stored = SetPasteBuffer::execute(invocation.server, std::move(request)); !stored) {
        return std::unexpected(stored.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
