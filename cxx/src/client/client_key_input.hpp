#pragma once

#include "key_table/key_table_catalog.hpp"

#include <tmux_cxx/identity.hpp>
#include <tmux_cxx/session/session_prefix_keys.hpp>

#include <cstddef>
#include <optional>
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
    /** @brief Reset the per-turn allowance while retaining queued bytes and key-table state. */
    void begin_routing_turn();
    /** @brief Report queued input deferred by the exhausted allowance, excluding program
     * backpressure. */
    bool routing_deferred() const;
    /** @brief Route controls despite closed program input; defer passthrough only while accepting
     * input lacks capacity.
     *
     * Absent capacity discards unbound program bytes; zero capacity retains them for retry.
     * Input admission or selection changes return before processing the retained suffix. */
    Result<key_table::KeyBindingEffect>
    route_pending(Server &server, ClientId client, ClientSelection selection,
                  const SessionPrefixKeys &prefix_keys,
                  std::optional<std::size_t> program_input_capacity,
                  const key_table::KeyTableCatalog &key_tables);

  private:
    static constexpr std::size_t maximum_pending_bytes = 65536; ///< Per-client input bound.
    static constexpr std::size_t maximum_routing_steps = 64; ///< Decisions per client daemon turn.
    static constexpr std::string_view root_table = "root";   ///< Default passthrough table.
    static constexpr std::string_view prefix_table = "prefix"; ///< One-shot command table.
    std::string active_table_{root_table}; ///< Table used by the next normalized key.
    std::string pending_bytes_;            ///< Ordered bytes not yet consumed or admitted.
    std::size_t remaining_routing_steps_ = maximum_routing_steps; ///< Shared across target changes.
};
} // namespace tmux_cxx
