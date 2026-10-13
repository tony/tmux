#include "terminal/terminal_state.hpp"

#include <cstdlib>

/** @brief Compare terminal cells, history boundaries, and replies across whole and fragmented
 * output.
 *
 * Parser revisions reflect feed calls and are excluded from comparison.
 * Unexpected exceptions abort this isolated terminal test. */
extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data, std::size_t size) noexcept try {
    if (size == 0 || size > 4096) {
        return 0;
    }
    const auto chunk_size = static_cast<std::size_t>(data[0] % 64) + 1;
    const std::string_view output(reinterpret_cast<const char *>(data + 1), size - 1);
    tmux_cxx::terminal::TerminalState whole({4, 8}, 32);
    tmux_cxx::terminal::TerminalState fragmented({4, 8}, 32);
    const auto accepted = whole.ingest_program_output(output);
    for (std::size_t offset = 0; offset < output.size(); offset += chunk_size) {
        if (!fragmented.ingest_program_output(output.substr(offset, chunk_size))) {
            return 0;
        }
    }
    if (!accepted) {
        return 0;
    }
    const auto first = whole.snapshot();
    const auto second = fragmented.snapshot();
    if (first.size != second.size || first.cursor != second.cursor ||
        first.cursor_visible != second.cursor_visible ||
        first.alternate_screen != second.alternate_screen || first.complete != second.complete ||
        first.cells != second.cells || first.history_lines != second.history_lines ||
        first.history_gap != second.history_gap ||
        whole.take_program_replies() != fragmented.take_program_replies()) {
        std::abort();
    }
    const auto first_history = whole.history_snapshot();
    const auto second_history = fragmented.history_snapshot();
    if (first_history.lines.size() != second_history.lines.size()) {
        std::abort();
    }
    for (std::size_t row = 0; row < first_history.lines.size(); ++row) {
        if (first_history.lines[row].continuation != second_history.lines[row].continuation ||
            first_history.lines[row].cells != second_history.lines[row].cells) {
            std::abort();
        }
    }
    return 0;
} catch (...) {
    std::abort();
}
