#pragma once

#include <tmux_cxx/protocol/tmux/stock_server_operation.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <string>
#include <vector>

namespace tmux_cxx {
class StockServerConnection;

/** @brief Describe the bounded session-name observation supported against stock tmux servers. */
struct ListStockServerSessionNames {
    /** @brief Argument-free stock session-name observation. */
    struct Request {};
    /** @brief Session names in the stock server's reported listing order. */
    using Response = std::vector<std::string>;
    /** @brief Declare the session operation identity, stock command, and required mode. */
    static constexpr protocol::tmux::StockServerOperationDescription description{
        "session.list_names", "list-sessions", protocol::tmux::ConnectionMode::command};
    /** @brief Query live session identities, then copy each UTF-8 name; concurrent removal can fail
     * the operation. */
    static Result<Response> execute(StockServerConnection &connection, Request request);
};
} // namespace tmux_cxx
