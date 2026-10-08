#pragma once

#include "key_table/key_table_catalog.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/session/session_prefix_keys.hpp>

#include <cstddef>
#include <string>
#include <string_view>

namespace tmux_cxx {
class Server;

/** @brief Retain one client's active key table and bounded undecided terminal input. */
class ClientKeyInput {
  public:
    /** @brief Return remaining space for bytes read from the client's terminal descriptor. */
    std::size_t read_capacity() const;
    /** @brief Append terminal bytes without discarding earlier input when the buffer is full. */
    Result<void> append(std::string_view bytes);
    /** @brief Route pending bytes and report whether the selected pane changed or the client
     * detached.
     */
    Result<key_table::KeyBindingEffect> route_pending(Server &server, ClientId client,
                                                      WindowId window, PaneId pane,
                                                      const SessionPrefixKeys &prefix_keys,
                                                      std::size_t program_input_capacity,
                                                      const key_table::KeyTableCatalog &key_tables);

  private:
    static constexpr std::size_t maximum_pending_bytes = 65536; ///< Per-client input bound.
    static constexpr std::string_view root_table = "root";      ///< Default passthrough table.
    static constexpr std::string_view prefix_table = "prefix";  ///< One-shot command table.
    std::string active_table_{root_table}; ///< Table used by the next normalized key.
    std::string pending_bytes_;            ///< Ordered bytes not yet consumed or admitted.
};
} // namespace tmux_cxx
