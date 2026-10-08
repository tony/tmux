#include "session_format.hpp"

namespace tmux_cxx::commands {
Result<std::string> format_session(const SessionSnapshot &session, std::string_view expression) {
    std::string formatted_session;
    while (!expression.empty()) {
        const auto variable_start = expression.find("#{");
        if (variable_start == expression.npos) {
            formatted_session += expression;
            break;
        }
        formatted_session += expression.substr(0, variable_start);
        expression.remove_prefix(variable_start + 2);
        const auto variable_end = expression.find('}');
        if (variable_end == expression.npos) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "unterminated format variable"});
        }
        const auto variable = expression.substr(0, variable_end);
        if (variable == "session_name") {
            formatted_session += session.name;
        } else if (variable == "session_id") {
            formatted_session += "$" + std::to_string(session.id.value);
        } else if (variable == "window_id") {
            formatted_session += "@" + std::to_string(session.window.value);
        } else if (variable == "pane_id") {
            formatted_session += "%" + std::to_string(session.pane.value);
        } else {
            return std::unexpected(
                TmuxError{TmuxErrorCode::invalid_argument, "unsupported format variable"});
        }
        expression.remove_prefix(variable_end + 1);
    }
    return formatted_session;
}
} // namespace tmux_cxx::commands
