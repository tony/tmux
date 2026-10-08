#include "pane/pane.hpp"

#include "server/state.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <unistd.h>

namespace tmux_cxx {
namespace {
constexpr std::size_t pty_io_byte_budget = 65536; ///< Per-direction work bound per turn.
} // namespace

void Pane::advance_pty_io() {
    std::size_t remaining_input_bytes = pty_io_byte_budget;
    (void)flush_program_input(remaining_input_bytes);
    if (!terminal_replies.empty()) {
        return;
    }
    std::array<char, 8192> pty_output_chunk{};
    std::size_t output_bytes_read = 0;
    while (output_bytes_read < pty_io_byte_budget) {
        auto read_bytes =
            ::read(pty.get(), pty_output_chunk.data(),
                   std::min(pty_output_chunk.size(), pty_io_byte_budget - output_bytes_read));
        if (read_bytes > 0) {
            output_bytes_read += static_cast<std::size_t>(read_bytes);
            (void)terminal.ingest_program_output(
                std::string_view(pty_output_chunk.data(), static_cast<std::size_t>(read_bytes)));
            terminal_replies = terminal.take_program_replies();
            output.append(
                std::string_view(pty_output_chunk.data(), static_cast<std::size_t>(read_bytes)));
            (void)flush_program_input(remaining_input_bytes);
            if (!terminal_replies.empty()) {
                return;
            }
        } else if (read_bytes == 0 || (read_bytes < 0 && errno == EIO)) {
            pty.reset();
            program_input.close_admission();
            terminal_replies.clear();
            return;
        } else if (read_bytes < 0 && errno == EINTR) {
            continue;
        } else {
            return;
        }
    }
}

Result<void> Pane::admit_program_input(std::string_view bytes) {
    if (!terminal_replies.empty()) {
        if (terminal_replies.size() > program_input.remaining_capacity()) {
            return std::unexpected(
                TmuxError{TmuxErrorCode::backpressure,
                          "earlier terminal replies await program input capacity"});
        }
        if (auto queued = program_input.admit(terminal_replies); !queued) {
            return queued;
        }
        terminal_replies.clear();
    }
    return program_input.admit(bytes);
}

Result<void> Pane::flush_program_input(std::size_t &remaining_bytes) {
    if (auto flushed = program_input.flush_to_pty(pty.get(), remaining_bytes); !flushed) {
        terminal_replies.clear();
        return flushed;
    }
    if (!terminal_replies.empty() &&
        terminal_replies.size() <= program_input.remaining_capacity()) {
        if (auto queued = program_input.admit(terminal_replies); !queued) {
            return queued;
        }
        terminal_replies.clear();
        return program_input.flush_to_pty(pty.get(), remaining_bytes);
    }
    return {};
}

std::size_t Pane::client_program_input_capacity() const {
    if (!program_input.accepting_input() ||
        terminal_replies.size() > program_input.remaining_capacity()) {
        return 0;
    }
    return program_input.remaining_capacity() - terminal_replies.size();
}

std::vector<std::pair<PaneId, int>> Server::pane_pty_descriptors() const {
    std::vector<std::pair<PaneId, int>> descriptors;
    for (const auto &[pane_id, pane] : state_->panes) {
        if (pane->pty.get() >= 0) {
            descriptors.emplace_back(pane_id, pane->pty.get());
        }
    }
    return descriptors;
}

void Server::advance_pane_pty_io(PaneId pane_id) {
    const auto pane_entry = state_->panes.find(pane_id);
    if (pane_entry != state_->panes.end()) {
        pane_entry->second->advance_pty_io();
    }
}

bool Server::pane_pty_write_pending(PaneId pane_id) const {
    const auto pane_entry = state_->panes.find(pane_id);
    return pane_entry != state_->panes.end() &&
           pane_entry->second->program_input.accepting_input() &&
           (pane_entry->second->program_input.delivery_pending() ||
            !pane_entry->second->terminal_replies.empty());
}

bool Server::pane_pty_read_blocked(PaneId pane_id) const {
    const auto pane_entry = state_->panes.find(pane_id);
    return pane_entry != state_->panes.end() &&
           pane_entry->second->program_input.accepting_input() &&
           !pane_entry->second->terminal_replies.empty();
}

} // namespace tmux_cxx
