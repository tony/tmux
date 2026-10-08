#include "session/session_options.hpp"

namespace tmux_cxx::session_options {
const options::OptionCatalog &catalog() {
    static constexpr options::OptionCatalog session_option_catalog{definitions};
    return session_option_catalog;
}

options::OptionTable global_defaults() { return options::OptionTable::defaults(catalog()); }

Result<SessionPrefixKeys> prefix_keys(const options::OptionTable &session_overrides,
                                      const options::OptionTable &global_options) {
    auto primary_prefix = session_overrides.key(prefix, &global_options);
    if (!primary_prefix) {
        return std::unexpected(primary_prefix.error());
    }
    auto secondary_prefix = session_overrides.key(prefix2, &global_options);
    if (!secondary_prefix) {
        return std::unexpected(secondary_prefix.error());
    }
    return SessionPrefixKeys{*primary_prefix, *secondary_prefix};
}
} // namespace tmux_cxx::session_options
