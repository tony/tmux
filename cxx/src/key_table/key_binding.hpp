#pragma once

#include "client/client_selection.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/key_table/key_code.hpp>
#include <tmux_cxx/protocol/native/operation.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <optional>
#include <string_view>

namespace tmux_cxx {
class Server;
/** @brief Model named tmux key tables and their operation-backed bindings. */
namespace key_table {
/** @brief Declare one key-table entry and the typed operation it reaches. */
struct KeyBindingDescription {
    std::string_view table;                  ///< Named table containing this binding.
    KeyCode key;                             ///< Normalized key selecting this binding.
    std::string_view command;                ///< Tmux command name used in errors and discovery.
    protocol::native::OperationId operation; ///< Shared typed operation required by the command.
};

/** @brief Supply the live server and invoking client's resolved selection to one matched binding.
 */
struct KeyBindingInvocation {
    Server &server;            ///< Authoritative server whose operation the binding invokes.
    ClientId client;           ///< Attached client whose active table matched the key.
    ClientSelection selection; ///< Session-owned membership and pane resolved before invocation.
    std::optional<KeyCode> prefix; ///< Effective primary prefix used by send-prefix.
};

/** @brief Report how one successful binding affects subsequent client terminal input. */
enum class KeyBindingEffect {
    client_selection_unchanged, ///< Keep routing terminal bytes to the resolved pane.
    client_selection_changed,   ///< Resolve the client's selected membership and pane again.
    pane_program_input_changed, ///< Resolve pane input capacity before routing further bytes.
    pane_mode_changed,          ///< Resolve the pane mode before interpreting retained keys.
    client_detached             ///< End input routing because the client attachment ended.
};
} // namespace key_table
} // namespace tmux_cxx
