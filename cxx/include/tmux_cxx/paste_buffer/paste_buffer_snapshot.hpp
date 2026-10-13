#pragma once

#include <cstdint>
#include <string>

namespace tmux_cxx {
/** @brief Copy server-global paste-buffer metadata without retaining its stored bytes. */
struct PasteBufferSnapshot {
    /** @brief Name resolved against the current server store. */
    std::string name;
    /** @brief Stored byte count, bounded to 256 KiB. */
    std::uint32_t byte_count;
    /** @brief Automatic buffers participate in default paste and eviction. */
    bool automatic;
    /** @brief Server mutation revision at the buffer's last replacement. */
    std::uint64_t revision;
};
} // namespace tmux_cxx
