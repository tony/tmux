#pragma once

#include <cstdint>

namespace tmux_cxx::protocol::native {
/** @brief Identify native server behavior independently of terminal feature flags. */
enum class Capability : std::uint64_t {
    session_lifecycle = 1,
    session_names = 2,
    window_links = 4,
    pane_input = 8,
    pane_raw_output = 16,
    pane_output_wait = 32,
    pane_terminal_text = 64,
    client_attachment = 128,
    stock_client_compatibility = 256,
    pane_layout = 512,
    paste_buffers = 1024,
    metadata_observation = 2048,
    session_options = 4096,
};
/** @brief Convert a declared native capability to its stable wire bit. */
constexpr std::uint64_t capability_mask(Capability capability) {
    return static_cast<std::uint64_t>(capability);
}
} // namespace tmux_cxx::protocol::native
