#include "window/window_name.hpp"

#include "text/utf8.hpp"

#include <algorithm>

namespace tmux_cxx {
bool valid_window_name(std::string_view name) {
    return !name.empty() && name.size() <= 128 && text::valid_utf8(name) &&
           std::ranges::none_of(
               name, /** @brief Reject ASCII control bytes in shared-window names. */
               [](unsigned char byte) { return byte < 0x20 || byte == 0x7f; });
}
} // namespace tmux_cxx
