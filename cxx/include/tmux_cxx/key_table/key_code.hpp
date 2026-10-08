#pragma once

#include <compare>
#include <cstdint>

namespace tmux_cxx::key_table {
/** @brief Carry one normalized tmux key identity independently of its terminal encoding. */
struct KeyCode {
    std::uint32_t value; ///< Normalized key value; initial input support covers single bytes.
    /** @brief Compare normalized key identities for table ordering and equality. */
    auto operator<=>(const KeyCode &) const = default;
};

/** @brief Normalize one compile-time ASCII letter to its control-key identity.
 *
 * The argument must be an ASCII letter.
 */
consteval KeyCode control_key(char letter) {
    if (letter >= 'A' && letter <= 'Z') {
        letter = static_cast<char>(letter - 'A' + 'a');
    }
    if (letter < 'a' || letter > 'z') {
        throw "control_key requires an ASCII letter";
    }
    return KeyCode{static_cast<std::uint32_t>(letter - 'a' + 1)};
}
} // namespace tmux_cxx::key_table
