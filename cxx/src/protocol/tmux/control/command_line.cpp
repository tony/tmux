#include "protocol/tmux/control/command_line.hpp"

#include "protocol/tmux/control/limits.hpp"

#include <utility>

namespace tmux_cxx::protocol::tmux::control {
namespace {
/** @brief Track the quoting rule active for the current control-command argument. */
enum class QuoteMode { unquoted, single_quoted, double_quoted };

/** @brief Test whether one byte separates unquoted control-command arguments. */
bool is_argument_separator(unsigned char byte) { return byte == ' ' || byte == '\t'; }

/** @brief Reject bytes that cannot occur in this declared command-line subset. */
bool is_rejected_control_byte(unsigned char byte) { return byte < 0x20 || byte == 0x7f; }
} // namespace

Result<std::vector<std::string>> parse_command_line(std::string_view line) {
    if (line.empty() || line.size() > maximum_line_bytes) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "control command line is empty or too long"});
    }

    std::vector<std::string> arguments;
    std::string argument;
    QuoteMode quote_mode = QuoteMode::unquoted;
    bool argument_started = false;
    bool escaped = false;
    for (const unsigned char byte : line) {
        if (escaped) {
            if (is_rejected_control_byte(byte)) {
                return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                                 "control command contains a control byte"});
            }
            argument.push_back(static_cast<char>(byte));
            argument_started = true;
            escaped = false;
            continue;
        }
        if (quote_mode == QuoteMode::single_quoted) {
            if (byte == '\'') {
                quote_mode = QuoteMode::unquoted;
            } else if (is_rejected_control_byte(byte)) {
                return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                                 "control command contains a control byte"});
            } else {
                argument.push_back(static_cast<char>(byte));
            }
            continue;
        }
        if (quote_mode == QuoteMode::double_quoted) {
            if (byte == '"') {
                quote_mode = QuoteMode::unquoted;
            } else if (byte == '\\') {
                escaped = true;
            } else if (is_rejected_control_byte(byte)) {
                return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                                 "control command contains a control byte"});
            } else {
                argument.push_back(static_cast<char>(byte));
            }
            continue;
        }
        if (is_argument_separator(byte)) {
            if (argument_started) {
                arguments.push_back(std::move(argument));
                argument.clear();
                argument_started = false;
            }
            continue;
        }
        if (byte == ';') {
            return std::unexpected(
                TmuxError{TmuxErrorCode::unsupported, "control command groups are not supported"});
        }
        if (byte == '\'') {
            quote_mode = QuoteMode::single_quoted;
            argument_started = true;
        } else if (byte == '"') {
            quote_mode = QuoteMode::double_quoted;
            argument_started = true;
        } else if (byte == '\\') {
            escaped = true;
            argument_started = true;
        } else if (is_rejected_control_byte(byte)) {
            return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                             "control command contains a control byte"});
        } else {
            argument.push_back(static_cast<char>(byte));
            argument_started = true;
        }
    }
    if (escaped || quote_mode != QuoteMode::unquoted) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "control command has an incomplete quote"});
    }
    if (argument_started) {
        arguments.push_back(std::move(argument));
    }
    if (arguments.empty() || arguments.size() > maximum_arguments) {
        return std::unexpected(TmuxError{TmuxErrorCode::invalid_argument,
                                         "control command has no arguments or too many arguments"});
    }
    return arguments;
}
} // namespace tmux_cxx::protocol::tmux::control
