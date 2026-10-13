#include "set_buffer_command.hpp"

#include <type_traits>

namespace tmux_cxx {
Result<SetBufferCommand::Request> SetBufferCommand::read(std::span<const std::string> arguments) {
    if (arguments.size() == 1 && !(arguments[0].size() > 1 && arguments[0].front() == '-')) {
        return Request{AddPasteBuffer::Request{arguments[0]}};
    }
    if (arguments.size() == 2 && arguments[0] == "--") {
        return Request{AddPasteBuffer::Request{arguments[1]}};
    }
    if (arguments.size() == 3 && arguments[0] == "-b" &&
        !(arguments[2].size() > 1 && arguments[2].front() == '-')) {
        return Request{SetPasteBuffer::Request{arguments[1], arguments[2]}};
    }
    if (arguments.size() == 4 && arguments[0] == "-b" && arguments[2] == "--") {
        return Request{SetPasteBuffer::Request{arguments[1], arguments[3]}};
    }
    return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                     "supported syntax: set-buffer [-b name] [--] data"});
}

Result<commands::CommandOutcome> SetBufferCommand::execute(commands::CommandInvocation &invocation,
                                                           Request request) {
    const auto stored = std::visit(
        /** @brief Invoke the operation selected by the parsed named or automatic request. */
        [&invocation](auto typed_request) -> Result<void> {
            using Operation =
                std::conditional_t<std::is_same_v<decltype(typed_request), AddPasteBuffer::Request>,
                                   AddPasteBuffer, SetPasteBuffer>;
            auto stored = Operation::execute(invocation.server, std::move(typed_request));
            return stored ? Result<void>{} : std::unexpected(stored.error());
        },
        std::move(request));
    if (!stored) {
        return std::unexpected(stored.error());
    }
    return commands::CommandOutcome{"", commands::ClientContinuation::exit};
}
} // namespace tmux_cxx
