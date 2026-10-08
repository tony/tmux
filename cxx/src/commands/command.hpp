#pragma once

#include <tmux_cxx/identity.hpp>

#include <string>

namespace tmux_cxx {
class Server;
/** @brief Parse and execute registered stock tmux commands through typed entity operations. */
namespace commands {
/** @brief Provide one stock command with its live server and invoking tmux client identity. */
struct CommandInvocation {
    Server &server;  ///< Server whose entities the command may observe or mutate.
    ClientId client; ///< Connection-lifetime client that invoked this command.
};

/** @brief State what the invoking client does after a command succeeds. */
enum class ClientContinuation {
    exit,    ///< Deliver command output and status, then close the command client.
    attached ///< Keep the identified ordinary client attached to its selected session.
};

/** @brief Carry command-owned display text and the invoking client's next lifecycle state. */
struct CommandOutcome {
    std::string output;              ///< Text delivered through the stock command-output stream.
    ClientContinuation continuation; ///< Client lifecycle selected by the typed command.
};
} // namespace commands
} // namespace tmux_cxx
