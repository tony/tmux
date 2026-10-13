#include "window/operations/set_window_option_command.hpp"

namespace tmux_cxx {
Result<SetWindowOptionCommand::Request>
SetWindowOptionCommand::read(std::span<const std::string> arguments) {
    return SetOptionCommand::read_window(arguments);
}
Result<commands::CommandOutcome>
SetWindowOptionCommand::execute(commands::CommandInvocation &invocation, Request request) {
    return SetOptionCommand::execute_window(invocation, std::move(request));
}
} // namespace tmux_cxx
