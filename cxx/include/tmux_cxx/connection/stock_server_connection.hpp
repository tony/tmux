#pragma once

#include <tmux_cxx/connection/stock_command_result.hpp>
#include <tmux_cxx/protocol/tmux/protocol_profile.hpp>
#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <memory>
#include <span>
#include <string>
#include <vector>

namespace tmux_cxx {
struct ListStockServerSessionNames;

/** @brief Expose typed bounded operations against one explicitly profiled stock tmux server. */
class StockServerConnection {
  public:
    /** @brief Select a declared profile and endpoint without contacting the stock server.
     * @throws std::invalid_argument When the profile or working directory is undeclared.
     */
    StockServerConnection(std::string socket_path,
                          const protocol::tmux::TmuxProtocolProfile &profile,
                          std::string working_directory = "/");
    /** @brief Release endpoint state without sending commands or stopping the stock server. */
    ~StockServerConnection();
    /** @brief Prevent sharing command admission and compatibility evidence through copies. */
    StockServerConnection(const StockServerConnection &) = delete;
    /** @brief Prevent assigning shared command admission and compatibility evidence. */
    StockServerConnection &operator=(const StockServerConnection &) = delete;
    /** @brief Transfer endpoint ownership and leave the source closed. */
    StockServerConnection(StockServerConnection &&) noexcept;
    /** @brief Replace endpoint ownership after releasing the previous local state. */
    StockServerConnection &operator=(StockServerConnection &&) noexcept;
    /** @brief Copy stock session names through the declared bounded query adapter. */
    Result<std::vector<std::string>> session_names();
    /** @brief Query `#{version}` and return the effective command-mode compatibility report. */
    Result<protocol::tmux::StockConnectionCompatibility> connection_compatibility();
    /** @brief Stop admitting commands without affecting the stock server or its sessions. */
    void close();

  private:
    friend struct ListStockServerSessionNames;
    struct StockServerEndpoint;
    /** @brief Exchange fixed operation-owned arguments over one fresh stock client connection. */
    Result<StockCommandResult> exchange_operation_command(std::span<const std::string> arguments);
    std::unique_ptr<StockServerEndpoint> endpoint_; ///< Owned endpoint policy and release evidence.
};
} // namespace tmux_cxx
