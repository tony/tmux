/*
 * Editor-derived defaults follow tmux.c at
 * 81f88f8517c9fc5371b56cf117530c6b477c96ac (tmux 3.7).
 *
 * Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
 * IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
 * OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */
#include "window/window_options.hpp"

#include <cstdlib>

namespace tmux_cxx::window_options {
const options::OptionCatalog &catalog() {
    static constexpr options::OptionCatalog window_option_catalog{definitions};
    return window_option_catalog;
}

WindowModeKeys mode_keys_from_editors(std::optional<std::string_view> visual,
                                      std::optional<std::string_view> editor) {
    const auto selected = visual ? visual : editor;
    if (!selected) {
        return WindowModeKeys::emacs;
    }
    const auto separator = selected->rfind('/');
    const auto basename = separator == selected->npos ? *selected : selected->substr(separator + 1);
    return basename.find("vi") == basename.npos ? WindowModeKeys::emacs : WindowModeKeys::vi;
}

options::OptionTable global_defaults() {
    auto defaults = options::OptionTable::defaults(catalog());
    const auto *visual = std::getenv("VISUAL");
    const auto *editor = visual == nullptr ? std::getenv("EDITOR") : nullptr;
    const auto value = mode_keys_from_editors(
        visual == nullptr ? std::nullopt : std::optional{std::string_view{visual}},
        editor == nullptr ? std::nullopt : std::optional{std::string_view{editor}});
    (void)defaults.set({&mode_keys, value});
    return defaults;
}

Result<WindowModeKeys> effective_mode_keys(const options::OptionTable &window_overrides,
                                           const options::OptionTable &global_options) {
    const auto value = window_overrides.effective(mode_keys, &global_options);
    if (!value) {
        return std::unexpected(value.error());
    }
    return std::get<WindowModeKeys>(*value);
}
} // namespace tmux_cxx::window_options
