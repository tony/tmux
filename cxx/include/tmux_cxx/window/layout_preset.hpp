#pragma once

#include <array>
#include <cstdint>
#include <string_view>
#include <utility>

namespace tmux_cxx {
/** @brief Select one named tmux arrangement without identifying a mutable layout tree. */
enum class WindowLayoutPreset : std::uint32_t {
    even_horizontal = 0,          ///< Divide columns equally in logical pane order.
    even_vertical = 1,            ///< Divide rows equally in logical pane order.
    main_horizontal = 2,          ///< Place the first pane above a secondary row.
    main_horizontal_mirrored = 3, ///< Place the first pane below a secondary row.
    main_vertical = 4,            ///< Place the first pane left of a secondary column.
    main_vertical_mirrored = 5,   ///< Place the first pane right of a secondary column.
    tiled = 6,                    ///< Arrange row-major panes in a near-square grid.
};

/** @brief Associate one typed preset with its exact stock command name. */
struct WindowLayoutPresetDescription {
    WindowLayoutPreset preset; ///< Closed arrangement exposed by native bindings.
    std::string_view name;     ///< Exact tmux spelling shared by command adapters.
};

inline constexpr std::array<WindowLayoutPresetDescription, 7> window_layout_presets{{
    {WindowLayoutPreset::even_horizontal, "even-horizontal"},
    {WindowLayoutPreset::even_vertical, "even-vertical"},
    {WindowLayoutPreset::main_horizontal, "main-horizontal"},
    {WindowLayoutPreset::main_horizontal_mirrored, "main-horizontal-mirrored"},
    {WindowLayoutPreset::main_vertical, "main-vertical"},
    {WindowLayoutPreset::main_vertical_mirrored, "main-vertical-mirrored"},
    {WindowLayoutPreset::tiled, "tiled"},
}}; ///< Immutable preset names; registration and runtime mutation are unnecessary.

static_assert(/** @brief Reject inconsistent enum identities or duplicate preset names at compile
                 time. */
              []() consteval {
                  for (std::size_t index = 0; index < window_layout_presets.size(); ++index) {
                      const auto &entry = window_layout_presets[index];
                      if (std::to_underlying(entry.preset) != index || entry.name.empty()) {
                          return false;
                      }
                      for (std::size_t previous = 0; previous < index; ++previous) {
                          if (entry.name == window_layout_presets[previous].name) {
                              return false;
                          }
                      }
                  }
                  return true;
              }());
} // namespace tmux_cxx
