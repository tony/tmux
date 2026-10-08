#pragma once

#include <tmux_cxx/identity.hpp>

#include <cstddef>
#include <cstdint>
#include <string>

namespace tmux_cxx {
inline constexpr std::size_t pane_output_retention_bytes =
    256U * 1024U; ///< Raw suffix retained independently for each pane.
/** @brief Identify one absolute raw-output position for one pane in one server lifetime. */
struct PaneOutputCursor {
    std::string server_instance; ///< Server lifetime that owns the observed pane.
    PaneId pane;                 ///< Pane whose raw byte stream establishes the offset.
    std::uint64_t offset;        ///< Absolute byte position following an observed prefix.
};

/** @brief Copy one retained pane-output suffix and its following absolute cursor. */
struct PaneOutputChunk {
    std::string bytes;     ///< Raw bytes beginning at the requested cursor.
    PaneOutputCursor next; ///< Cursor immediately following the copied bytes.
};
} // namespace tmux_cxx
