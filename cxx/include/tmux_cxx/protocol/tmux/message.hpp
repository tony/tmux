#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <optional>
#include <span>
#include <string_view>

namespace tmux_cxx::protocol::tmux {
/** @brief Name stock tmux message identities without conflating them with native API operations. */
enum class TmuxMessage : std::uint32_t {
    version = 12,
    identify_flags = 100,
    identify_term = 101,
    identify_ttyname = 102,
    identify_oldcwd = 103,
    identify_stdin = 104,
    identify_environ = 105,
    identify_done = 106,
    identify_clientpid = 107,
    identify_cwd = 108,
    identify_features = 109,
    identify_stdout = 110,
    identify_longflags = 111,
    identify_terminfo = 112,
    command = 200,
    detach = 201,
    detachkill = 202,
    exit = 203,
    exited = 204,
    exiting = 205,
    lock = 206,
    ready = 207,
    resize = 208,
    shell = 209,
    shutdown = 210,
    old_stderr = 211,
    old_stdin = 212,
    old_stdout = 213,
    suspend = 214,
    unlock = 215,
    wakeup = 216,
    exec = 217,
    flags = 218,
    read_open = 300,
    read_data = 301,
    read_done = 302,
    write_open = 303,
    write_data = 304,
    write_ready = 305,
    write_close = 306,
    read_cancel = 307,
    write_done = 308,
};

/** @brief Describe roles sharing an identical stock message contract. */
enum class TmuxMessageDirection { client_to_server, server_to_client, bidirectional };
/** @brief Describe descriptor ownership required by one stock message. */
enum class TmuxDescriptorRule { forbidden, required };
/** @brief Name payload layouts whose host-ABI validation belongs to the stock codec. */
enum class TmuxPayloadLayout {
    empty,
    host_u32,
    host_pid,
    host_u64,
    nul_terminated_bytes,
    command_arguments,
    exit_status,
    host_u32_pair,
    host_u32_triple,
    stream_data,
    host_u32_pair_with_optional_path,
    host_u32_triple_with_optional_path,
    two_nul_terminated_bytes,
    opaque_bytes,
};

/** @brief Declare one role's payload and descriptor contract for a stock message. */
struct TmuxMessageContract {
    /** @brief Host-ABI payload layout validated before role-specific behavior. */
    TmuxPayloadLayout payload;
    /** @brief Whether this frame must carry a transferred descriptor. */
    TmuxDescriptorRule descriptor;
};

/** @brief Declare one stock message's identity and role-specific wire contracts. */
struct TmuxMessageDescription {
    TmuxMessage message;   ///< Stable wire identity in the selected profile.
    std::string_view name; ///< Source-level name retained for diagnostics and evidence.
    std::optional<TmuxMessageContract> from_client; ///< Contract when a client sends it.
    std::optional<TmuxMessageContract> from_server; ///< Contract when a server sends it.

    /** @brief Expand a shared role contract into its permitted sender slots. */
    constexpr TmuxMessageDescription(TmuxMessage message, std::string_view name,
                                     TmuxMessageDirection direction, TmuxPayloadLayout payload,
                                     TmuxDescriptorRule descriptor)
        : message(message), name(name),
          from_client(direction == TmuxMessageDirection::server_to_client
                          ? std::nullopt
                          : std::optional(TmuxMessageContract{payload, descriptor})),
          from_server(direction == TmuxMessageDirection::client_to_server
                          ? std::nullopt
                          : std::optional(TmuxMessageContract{payload, descriptor})) {}

    /** @brief Retain distinct contracts for messages whose payload depends on sender role. */
    constexpr TmuxMessageDescription(TmuxMessage message, std::string_view name,
                                     std::optional<TmuxMessageContract> from_client,
                                     std::optional<TmuxMessageContract> from_server)
        : message(message), name(name), from_client(from_client), from_server(from_server) {}
};

/** @brief Convert a typed stock message identity to its host-order wire number. */
constexpr std::uint32_t message_number(TmuxMessage message) {
    return static_cast<std::uint32_t>(message);
}

/** @brief Declare every active and retained protocol-8 message contract for both sender roles. */
inline constexpr std::array stock_v8_messages{
    TmuxMessageDescription{TmuxMessage::version, "MSG_VERSION", TmuxMessageDirection::bidirectional,
                           TmuxPayloadLayout::empty, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_flags, "MSG_IDENTIFY_FLAGS",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u32,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_term, "MSG_IDENTIFY_TERM",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_ttyname, "MSG_IDENTIFY_TTYNAME",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_oldcwd, "MSG_IDENTIFY_OLDCWD", std::nullopt,
                           std::nullopt},
    TmuxMessageDescription{TmuxMessage::identify_stdin, "MSG_IDENTIFY_STDIN",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::required},
    TmuxMessageDescription{TmuxMessage::identify_environ, "MSG_IDENTIFY_ENVIRON",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_done, "MSG_IDENTIFY_DONE",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_clientpid, "MSG_IDENTIFY_CLIENTPID",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_pid,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_cwd, "MSG_IDENTIFY_CWD",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_features, "MSG_IDENTIFY_FEATURES",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u32,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_stdout, "MSG_IDENTIFY_STDOUT",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::required},
    TmuxMessageDescription{TmuxMessage::identify_longflags, "MSG_IDENTIFY_LONGFLAGS",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u64,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::identify_terminfo, "MSG_IDENTIFY_TERMINFO",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::command, "MSG_COMMAND",
                           TmuxMessageDirection::client_to_server,
                           TmuxPayloadLayout::command_arguments, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::detach, "MSG_DETACH",
                           TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::detachkill, "MSG_DETACHKILL",
                           TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::exit, "MSG_EXIT", TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::exit_status, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::exited, "MSG_EXITED",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::exiting, "MSG_EXITING",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::lock, "MSG_LOCK", TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::nul_terminated_bytes, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::ready, "MSG_READY", TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::empty, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::resize, "MSG_RESIZE",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{
        TmuxMessage::shell, "MSG_SHELL",
        TmuxMessageContract{TmuxPayloadLayout::empty, TmuxDescriptorRule::forbidden},
        TmuxMessageContract{TmuxPayloadLayout::nul_terminated_bytes,
                            TmuxDescriptorRule::forbidden}},
    TmuxMessageDescription{TmuxMessage::shutdown, "MSG_SHUTDOWN",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::exit_status,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::old_stderr, "MSG_OLDSTDERR",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::opaque_bytes,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::old_stdin, "MSG_OLDSTDIN",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::opaque_bytes,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::old_stdout, "MSG_OLDSTDOUT",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::opaque_bytes,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::suspend, "MSG_SUSPEND",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::unlock, "MSG_UNLOCK",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::wakeup, "MSG_WAKEUP",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::empty,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::exec, "MSG_EXEC", TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::two_nul_terminated_bytes,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::flags, "MSG_FLAGS", TmuxMessageDirection::server_to_client,
                           TmuxPayloadLayout::host_u64, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{
        TmuxMessage::read_open, "MSG_READ_OPEN", TmuxMessageDirection::server_to_client,
        TmuxPayloadLayout::host_u32_pair_with_optional_path, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::read_data, "MSG_READ",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::stream_data,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::read_done, "MSG_READ_DONE",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u32_pair,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{
        TmuxMessage::write_open, "MSG_WRITE_OPEN", TmuxMessageDirection::server_to_client,
        TmuxPayloadLayout::host_u32_triple_with_optional_path, TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::write_data, "MSG_WRITE",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::stream_data,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::write_ready, "MSG_WRITE_READY",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u32_pair,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::write_close, "MSG_WRITE_CLOSE",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::host_u32,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::read_cancel, "MSG_READ_CANCEL",
                           TmuxMessageDirection::server_to_client, TmuxPayloadLayout::host_u32,
                           TmuxDescriptorRule::forbidden},
    TmuxMessageDescription{TmuxMessage::write_done, "MSG_WRITE_DONE",
                           TmuxMessageDirection::client_to_server, TmuxPayloadLayout::host_u32_pair,
                           TmuxDescriptorRule::forbidden},
};

/** @brief Find one declared message without accepting an unknown numeric identity. */
constexpr const TmuxMessageDescription *
message_description(std::span<const TmuxMessageDescription> catalog, std::uint32_t number) {
    const auto found = std::ranges::find(
        catalog, number,
        /** @brief Project one declared message to its stable numeric wire identity. */
        [](const auto &description) { return message_number(description.message); });
    return found == catalog.end() ? nullptr : &*found;
}
} // namespace tmux_cxx::protocol::tmux
