#pragma once

#include "paste_buffer/paste_buffer.hpp"
#include <tmux_cxx/paste_buffer/paste_buffer_snapshot.hpp>
#include <tmux_cxx/tmux_error.hpp>

#include <optional>
#include <vector>

namespace tmux_cxx {
/** @brief Own newest-first named and automatic paste buffers with independent retention bounds. */
class PasteBufferStore {
  public:
    /** @brief Reserve the bounded store before admitting any buffer mutation. */
    PasteBufferStore();
    /** @brief Replace named bytes at the newest position; empty bytes preserve all store state. */
    Result<bool> set(std::string name, std::string bytes, std::uint64_t revision);
    /** @brief Add collision-free automatic bytes, evicting only the oldest automatic buffer. */
    Result<std::optional<PasteBufferSnapshot>> add(std::string bytes, std::uint64_t revision);
    /** @brief Remove one live buffer without changing the order of remaining entries. */
    Result<void> erase(std::string_view name);
    /** @brief Borrow one buffer by name until any store mutation invalidates the reference. */
    const PasteBuffer *find(std::string_view name) const;
    /** @brief Borrow the newest automatic buffer until the next store mutation. */
    const PasteBuffer *top() const;
    /** @brief Copy buffer metadata in newest-first creation order without copying retained bytes.
     */
    std::vector<PasteBufferSnapshot> snapshots() const;
    /** @brief Discard retained bytes while preserving automatic name allocation history. */
    void clear();

  private:
    std::vector<PasteBuffer>
        buffers_; ///< Owned newest-first entries, including named replacements.
    std::uint64_t next_automatic_index_ =
        0; ///< Monotonic candidate suffix, skipping occupied names.
};
} // namespace tmux_cxx
