#pragma once

#include "option/option_table.hpp"

#include <tmux_cxx/window/mode_keys.hpp>

#include <array>
#include <optional>
#include <string_view>

namespace tmux_cxx::window_options {
inline constexpr options::OptionDefinition mode_keys{
    "mode-keys", options::OptionScope::window, options::OptionValueType::mode_keys,
    WindowModeKeys::emacs}; ///< Window-scoped copy-mode convention with tmux's emacs default.
inline constexpr std::array<const options::OptionDefinition *, 1> definitions{
    &mode_keys}; ///< Stable window option definitions used by parsing and defaults.
static_assert(options::valid_scoped_option_definitions(definitions, options::OptionScope::window));

/** @brief Borrow the immutable catalog of implemented window options. */
const options::OptionCatalog &catalog();
/** @brief Resolve editor-derived mode keys using VISUAL precedence and tmux's literal basename
 * match. */
WindowModeKeys mode_keys_from_editors(std::optional<std::string_view> visual,
                                      std::optional<std::string_view> editor);
/** @brief Construct global window defaults from the catalog and startup editor environment. */
options::OptionTable global_defaults();
/** @brief Resolve typed mode keys through local window overrides and live global defaults. */
Result<WindowModeKeys> effective_mode_keys(const options::OptionTable &window_overrides,
                                           const options::OptionTable &global_options);
} // namespace tmux_cxx::window_options
