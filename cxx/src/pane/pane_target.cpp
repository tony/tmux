#include "pane/pane_target.hpp"

#include <charconv>

namespace tmux_cxx {
std::optional<PaneId> parse_pane_target(std::string_view target) {
    if (target.size() < 2 || target.front() != '%') {
        return std::nullopt;
    }
    std::uint64_t value = 0;
    const auto [end, error] =
        std::from_chars(target.data() + 1, target.data() + target.size(), value);
    if (error != std::errc{} || end != target.data() + target.size() || value == 0) {
        return std::nullopt;
    }
    return PaneId{value};
}
} // namespace tmux_cxx
