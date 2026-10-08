#pragma once

#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>

namespace tmux_cxx::protocol::tmux {
/** @brief Declare adapter support independently of mandatory wire rules and detected releases. */
class TmuxCompatibilityPolicy {
  public:
    static constexpr std::string_view baseline_release =
        "3.4"; ///< First complete-baseline target; not a passing claim.
    /** @brief Assess one stock role and mode without inventing release or verification evidence. */
    StockConnectionCompatibility assess(const TmuxProtocolProfile &profile,
                                        ProtocolDirection direction, ConnectionMode mode) const;
    /** @brief Assess a stock server using release evidence returned over the selected connection.
     */
    StockConnectionCompatibility assess_stock_server(const TmuxProtocolProfile &profile,
                                                     std::string_view release,
                                                     ConnectionMode mode) const;
};
} // namespace tmux_cxx::protocol::tmux
