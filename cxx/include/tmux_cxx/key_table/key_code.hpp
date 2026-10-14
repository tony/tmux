#pragma once

#include <compare>
#include <cstdint>

namespace tmux_cxx::key_table {
/** @brief Preserve closed combinations of terminal modifiers independently of byte encoding. */
enum class KeyModifier : std::uint8_t {
    none = 0,
    shift = 1,
    meta = 2,
    shift_meta = 3,
    control = 4,
    shift_control = 5,
    meta_control = 6,
    shift_meta_control = 7,
};

/** @brief Combine independent terminal key modifiers. */
constexpr KeyModifier operator|(KeyModifier left, KeyModifier right) {
    return static_cast<KeyModifier>(static_cast<std::uint8_t>(left) |
                                    static_cast<std::uint8_t>(right));
}

/** @brief Carry one normalized tmux key identity independently of its terminal encoding. */
struct KeyCode {
    std::uint32_t value; ///< Unicode scalar, existing C0 identity, or named special key.
    KeyModifier modifiers = KeyModifier::none; ///< Modifiers participating in binding identity.
    /** @brief Compare normalized key identities for table ordering and equality. */
    auto operator<=>(const KeyCode &) const = default;
};

inline constexpr KeyCode escape_key{0x1b};    ///< Literal Escape, distinct from a Meta modifier.
inline constexpr KeyCode up_key{0x110000};    ///< Up arrow; special values follow Unicode scalars.
inline constexpr KeyCode down_key{0x110001};  ///< Down arrow.
inline constexpr KeyCode right_key{0x110002}; ///< Right arrow.
inline constexpr KeyCode left_key{0x110003};  ///< Left arrow.
inline constexpr KeyCode home_key{0x110004};  ///< Home.
inline constexpr KeyCode end_key{0x110005};   ///< End.
inline constexpr KeyCode page_up_key{0x110006};   ///< Page Up.
inline constexpr KeyCode page_down_key{0x110007}; ///< Page Down.

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
