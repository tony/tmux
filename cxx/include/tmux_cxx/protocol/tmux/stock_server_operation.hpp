#pragma once

#include <tmux_cxx/protocol/tmux/stock_connection_compatibility.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <concepts>
#include <string_view>
#include <utility>

namespace tmux_cxx {
class StockServerConnection;
namespace protocol::tmux {
/** @brief Declare one typed C++ operation adapted to a stock tmux server command. */
struct StockServerOperationDescription {
    std::string_view name;    ///< Stable tmux operation identity used in failures.
    std::string_view command; ///< Stock command spelling fixed by the operation adapter.
    ConnectionMode mode;      ///< Stock connection mode required by this adapter.
};

/** @brief Require a typed request and result for an operation adapted to a stock server. */
template <class Operation>
concept RegisteredStockServerOperation =
    requires(StockServerConnection &connection, typename Operation::Request request) {
        typename Operation::Response;
        { Operation::description } -> std::same_as<const StockServerOperationDescription &>;
        {
            Operation::execute(connection, std::move(request))
        } -> std::same_as<Result<typename Operation::Response>>;
    };
} // namespace protocol::tmux
} // namespace tmux_cxx
