/*
 * Automatic naming, selection, and replacement ordering follow tmux paste.c
 * at 81f88f8517c9fc5371b56cf117530c6b477c96ac (tmux 3.7).
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
#include "paste_buffer/paste_buffer_store.hpp"

#include <algorithm>
#include <iterator>
#include <limits>
#include <ranges>

namespace tmux_cxx {
PasteBufferStore::PasteBufferStore() {
    buffers_.reserve(paste_buffer_named_limit + paste_buffer_automatic_limit);
}

Result<bool> PasteBufferStore::set(std::string name, std::string bytes, std::uint64_t revision) {
    if (!valid_paste_buffer_name(name) || bytes.size() > paste_buffer_byte_limit) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name or size"});
    }
    if (bytes.empty()) {
        return false;
    }
    const auto existing = std::ranges::find(buffers_, name, &PasteBuffer::name);
    const auto named_count =
        buffers_.size() -
        static_cast<std::size_t>(std::ranges::count(buffers_, true, &PasteBuffer::automatic));
    if (named_count == paste_buffer_named_limit &&
        (existing == buffers_.end() || existing->automatic)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "named paste-buffer store is full"});
    }
    if (existing != buffers_.end()) {
        buffers_.erase(existing);
    }
    buffers_.insert(buffers_.begin(),
                    PasteBuffer{std::move(name), std::move(bytes), false, revision});
    return true;
}

Result<std::optional<PasteBufferSnapshot>> PasteBufferStore::add(std::string bytes,
                                                                 std::uint64_t revision) {
    if (bytes.size() > paste_buffer_byte_limit) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "paste buffer exceeds byte capacity"});
    }
    if (bytes.empty()) {
        return std::nullopt;
    }
    auto next_index = next_automatic_index_;
    std::string name;
    do {
        if (next_index == std::numeric_limits<std::uint64_t>::max()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure, "automatic paste-buffer names exhausted"});
        }
        name = "buffer" + std::to_string(next_index++);
    } while (find(name) != nullptr);
    PasteBufferSnapshot snapshot{name, static_cast<std::uint32_t>(bytes.size()), true, revision};
    const auto automatic_count =
        static_cast<std::size_t>(std::ranges::count(buffers_, true, &PasteBuffer::automatic));
    if (automatic_count == paste_buffer_automatic_limit) {
        const auto oldest =
            std::ranges::find(buffers_ | std::views::reverse, true, &PasteBuffer::automatic);
        buffers_.erase(std::prev(oldest.base()));
    }
    buffers_.insert(buffers_.begin(),
                    PasteBuffer{std::move(name), std::move(bytes), true, revision});
    next_automatic_index_ = next_index;
    return std::optional{std::move(snapshot)};
}

Result<void> PasteBufferStore::erase(std::string_view name) {
    if (!valid_paste_buffer_name(name)) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "invalid paste-buffer name"});
    }
    const auto existing = std::ranges::find(buffers_, name, &PasteBuffer::name);
    if (existing == buffers_.end()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::not_found, "paste buffer no longer exists"});
    }
    buffers_.erase(existing);
    return {};
}

const PasteBuffer *PasteBufferStore::find(std::string_view name) const {
    const auto buffer = std::ranges::find(buffers_, name, &PasteBuffer::name);
    return buffer == buffers_.end() ? nullptr : &*buffer;
}

const PasteBuffer *PasteBufferStore::top() const {
    const auto buffer = std::ranges::find(buffers_, true, &PasteBuffer::automatic);
    return buffer == buffers_.end() ? nullptr : &*buffer;
}

std::vector<PasteBufferSnapshot> PasteBufferStore::snapshots() const {
    std::vector<PasteBufferSnapshot> snapshots;
    snapshots.reserve(buffers_.size());
    for (const auto &buffer : buffers_) {
        snapshots.push_back({buffer.name, static_cast<std::uint32_t>(buffer.bytes.size()),
                             buffer.automatic, buffer.revision});
    }
    return snapshots;
}

void PasteBufferStore::clear() { buffers_.clear(); }
} // namespace tmux_cxx
