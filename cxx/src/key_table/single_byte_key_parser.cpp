#include "key_table/single_byte_key_parser.hpp"

namespace tmux_cxx::key_table {
Result<std::optional<KeyCode>> parse_single_byte_key(std::string_view text) {
    if (text == "None") {
        return std::optional<KeyCode>{};
    }
    if (text.size() == 1 && static_cast<unsigned char>(text.front()) <= 0x7fU) {
        return KeyCode{static_cast<unsigned char>(text.front())};
    }
    if (text.size() == 3 && (text[0] == 'C' || text[0] == 'c') && text[1] == '-') {
        auto letter = text[2];
        if (letter >= 'A' && letter <= 'Z') {
            letter = static_cast<char>(letter - 'A' + 'a');
        }
        if (letter >= 'a' && letter <= 'z') {
            return KeyCode{static_cast<std::uint32_t>(letter - 'a' + 1)};
        }
    }
    return std::unexpected(
        TmuxError{TmuxErrorCode::invalid_argument,
                  "unsupported key; expected None, one ASCII byte, or C-a through C-z"});
}
} // namespace tmux_cxx::key_table
