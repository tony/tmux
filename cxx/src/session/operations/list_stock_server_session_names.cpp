#include <tmux_cxx/connection/stock_server_connection.hpp>
#include <tmux_cxx/session/operations/list_stock_server_session_names.hpp>

#include "text/utf8.hpp"

#include <algorithm>
#include <array>
#include <cctype>

namespace tmux_cxx {
namespace {
constexpr std::size_t stock_session_count_limit = 4096; ///< Bound per-operation remote fan-out.
constexpr std::size_t stock_session_name_limit = 65536; ///< Bound one copied stock session name.

/** @brief Attach one typed operation identity to a lower-level stock transport failure. */
TmuxError attach_operation_identity(TmuxError error) {
    error.operation = ListStockServerSessionNames::description.name;
    return error;
}

/** @brief Require a successful stock query and return its byte-preserving standard output. */
Result<protocol::Bytes> require_successful_query(StockCommandResult result) {
    if (result.exit_status != 0 || !result.standard_error.empty()) {
        return std::unexpected(TmuxError{
            TmuxErrorCode::protocol, "stock session query returned an unsuccessful result",
            std::string(ListStockServerSessionNames::description.name)});
    }
    return std::move(result.standard_output);
}

/** @brief Parse bounded newline-delimited stock session identities without interpreting names. */
Result<std::vector<std::string>> parse_session_identities(const protocol::Bytes &bytes) {
    std::vector<std::string> identities;
    std::size_t offset = 0;
    while (offset < bytes.size()) {
        const auto newline = std::find(bytes.begin() + static_cast<std::ptrdiff_t>(offset),
                                       bytes.end(), static_cast<std::uint8_t>('\n'));
        if (newline == bytes.end() || identities.size() == stock_session_count_limit) {
            return std::unexpected(TmuxError{
                TmuxErrorCode::protocol, "stock session identity list is malformed or excessive",
                std::string(ListStockServerSessionNames::description.name)});
        }
        const auto end = static_cast<std::size_t>(newline - bytes.begin());
        std::string identity(bytes.begin() + static_cast<std::ptrdiff_t>(offset), newline);
        if (identity.size() < 2 || identity.front() != '$' ||
            !std::ranges::all_of(
                identity.substr(1),
                /** @brief Require decimal digits after tmux's session-identity sigil. */
                [](unsigned char character) { return std::isdigit(character) != 0; }) ||
            std::ranges::find(identities, identity) != identities.end()) {
            return std::unexpected(TmuxError{
                TmuxErrorCode::protocol, "stock server returned an invalid session identity",
                std::string(ListStockServerSessionNames::description.name)});
        }
        identities.push_back(std::move(identity));
        offset = end + 1;
    }
    return identities;
}

/** @brief Decode one name query while preserving embedded line breaks and removing its delimiter.
 */
Result<std::string> parse_session_name(protocol::Bytes bytes) {
    if (bytes.empty() || bytes.back() != '\n') {
        return std::unexpected(TmuxError{
            TmuxErrorCode::protocol, "stock session name response lacks its record delimiter",
            std::string(ListStockServerSessionNames::description.name)});
    }
    bytes.pop_back();
    std::string name(bytes.begin(), bytes.end());
    if (name.size() > stock_session_name_limit || !text::valid_utf8(name) ||
        name.find('\0') != std::string::npos) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::protocol, "stock server returned an invalid session name",
                      std::string(ListStockServerSessionNames::description.name)});
    }
    return name;
}
} // namespace

Result<ListStockServerSessionNames::Response>
ListStockServerSessionNames::execute(StockServerConnection &connection, Request) {
    auto compatibility = connection.connection_compatibility();
    if (!compatibility) {
        return std::unexpected(attach_operation_identity(compatibility.error()));
    }
    if (compatibility->support == protocol::tmux::CapabilitySupport::unsupported ||
        std::ranges::find(compatibility->commands, description.command) ==
            compatibility->commands.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::unsupported,
                                         "stock server session listing is not supported",
                                         std::string(description.name)});
    }

    const std::array<std::string, 3> list_arguments{"list-sessions", "-F", "#{session_id}"};
    auto list_result = connection.exchange_operation_command(list_arguments);
    if (!list_result) {
        return std::unexpected(attach_operation_identity(list_result.error()));
    }
    auto list_output = require_successful_query(std::move(*list_result));
    if (!list_output) {
        return std::unexpected(list_output.error());
    }
    auto identities = parse_session_identities(*list_output);
    if (!identities) {
        return std::unexpected(identities.error());
    }

    Response names;
    names.reserve(identities->size());
    for (const auto &identity : *identities) {
        const std::array<std::string, 5> name_arguments{"display-message", "-p", "-t", identity,
                                                        "#{session_name}"};
        auto name_result = connection.exchange_operation_command(name_arguments);
        if (!name_result) {
            return std::unexpected(attach_operation_identity(name_result.error()));
        }
        auto name_output = require_successful_query(std::move(*name_result));
        if (!name_output) {
            return std::unexpected(name_output.error());
        }
        auto name = parse_session_name(std::move(*name_output));
        if (!name) {
            return std::unexpected(name.error());
        }
        names.push_back(std::move(*name));
    }
    return names;
}

Result<std::vector<std::string>> StockServerConnection::session_names() {
    return ListStockServerSessionNames::execute(*this, {});
}
} // namespace tmux_cxx
