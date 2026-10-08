#pragma once

#include "option/option_table.hpp"

#include <tmux_cxx/session/session_prefix_keys.hpp>

#include <array>

namespace tmux_cxx::session_options {
/** @brief Define the primary session-prefix key with tmux's C-b default. */
inline constexpr options::OptionDefinition prefix{"prefix", options::OptionScope::session,
                                                  options::OptionValueType::key,
                                                  key_table::control_key('b')};
/** @brief Define the optional secondary session-prefix key with tmux's None default. */
inline constexpr options::OptionDefinition prefix2{"prefix2", options::OptionScope::session,
                                                   options::OptionValueType::key, std::monostate{}};
/** @brief Keep session option definitions ordered for deterministic discovery and defaults. */
inline constexpr std::array<const options::OptionDefinition *, 2> definitions{&prefix, &prefix2};
static_assert(options::valid_scoped_option_definitions(definitions, options::OptionScope::session));

/** @brief Borrow the immutable catalog of implemented session options. */
const options::OptionCatalog &catalog();
/** @brief Construct the server-global session options from catalog defaults. */
options::OptionTable global_defaults();
/** @brief Resolve one session's prefix keys through local overrides and global values. */
Result<SessionPrefixKeys> prefix_keys(const options::OptionTable &session_overrides,
                                      const options::OptionTable &global_options);
} // namespace tmux_cxx::session_options
