#pragma once

#include "option/option_table.hpp"
#include "text/utf8.hpp"

#include <tmux_cxx/identity.hpp>

#include <map>
#include <string>
#include <string_view>

namespace tmux_cxx {
/** @brief Own session naming, indexed window memberships, and the selected membership. */
struct Session {
    SessionId id;     ///< Stable identity allocated by this server.
    std::string name; ///< Unique session name validated before mutation.
    std::map<std::uint32_t, WindowLinkId>
        windows;                  ///< Memberships ordered by this session's window indices.
    WindowLinkId current;         ///< Selected membership; zero only during final removal.
    options::OptionTable options; ///< Session-local option overrides inherited from the server.
    std::uint64_t revision;       ///< Most recent session mutation in the server revision sequence.
};
/** @brief Reject invalid UTF-8, empty, oversized, or ambiguous tmux session names. */
inline bool valid_session_name(std::string_view name) {
    return !name.empty() && name.size() <= 128 && name.find_first_of(":.\n\r") == name.npos &&
           name.find('\0') == name.npos && text::valid_utf8(name);
}
} // namespace tmux_cxx
