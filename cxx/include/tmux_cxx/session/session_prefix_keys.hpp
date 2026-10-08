#pragma once

#include <tmux_cxx/key_table/key_code.hpp>

#include <optional>

namespace tmux_cxx {
/** @brief Carry the effective primary and secondary prefix keys for one session. */
struct SessionPrefixKeys {
    std::optional<key_table::KeyCode> prefix;  ///< Primary key entering the prefix table.
    std::optional<key_table::KeyCode> prefix2; ///< Secondary key entering the prefix table.
    /** @brief Report whether a normalized key enters this session's prefix table. */
    constexpr bool contains(key_table::KeyCode key) const {
        return prefix == key || prefix2 == key;
    }
};
} // namespace tmux_cxx
