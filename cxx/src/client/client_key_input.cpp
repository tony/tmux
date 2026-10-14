#include "client/client_key_input.hpp"

#include <tmux_cxx/pane/operations/send_pane_input.hpp>

#include <algorithm>

namespace tmux_cxx {
std::size_t ClientKeyInput::read_capacity() const {
    return maximum_pending_bytes - pending_bytes_.size();
}

Result<void> ClientKeyInput::append(std::string_view bytes) {
    if (bytes.size() > read_capacity()) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::backpressure, "client terminal input buffer is full"});
    }
    pending_bytes_.append(bytes);
    return {};
}

void ClientKeyInput::begin_routing_turn() { remaining_routing_steps_ = maximum_routing_steps; }

bool ClientKeyInput::routing_deferred() const {
    return remaining_routing_steps_ == 0 && !pending_bytes_.empty();
}

bool ClientKeyInput::failed_action_consumed() const { return failed_action_consumed_; }

std::optional<TerminalKeyDecoder::Clock::time_point> ClientKeyInput::next_key_deadline() const {
    return decoder_.next_deadline();
}

Result<key_table::KeyBindingEffect> ClientKeyInput::route_pending(
    Server &server, ClientId client, ClientSelection selection,
    const SessionPrefixKeys &prefix_keys, std::optional<std::size_t> program_input_capacity,
    const key_table::KeyTableCatalog &key_tables, std::string_view pane_table,
    TerminalKeyDecoder::Clock::time_point now) {
    failed_action_consumed_ = false;
    key_table::KeyBindingInvocation invocation{server, client, selection, prefix_keys.prefix};
    while (!pending_bytes_.empty()) {
        if (remaining_routing_steps_ == 0) {
            return key_table::KeyBindingEffect::client_selection_unchanged;
        }
        const auto decoded = decoder_.peek(pending_bytes_, now);
        if (decoded.status == TerminalKeyStatus::pending) {
            return key_table::KeyBindingEffect::client_selection_unchanged;
        }
        if (!prefix_pending_ && decoded.key && prefix_keys.contains(*decoded.key)) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, decoded.byte_count);
            decoder_.consume();
            prefix_pending_ = true;
            continue;
        }
        const bool was_prefix = prefix_pending_;
        const auto selected_table = was_prefix ? prefix_table : pane_table;
        prefix_pending_ = false;
        Result<std::optional<key_table::KeyBindingEffect>> binding =
            std::optional<key_table::KeyBindingEffect>{};
        if (decoded.key) {
            binding = key_tables.invoke(selected_table, *decoded.key, invocation);
            if (binding && !*binding && was_prefix) {
                binding = key_tables.invoke(root_table, *decoded.key, invocation);
            }
        }
        if (!binding) {
            if (binding.error().code == TmuxErrorCode::backpressure) {
                prefix_pending_ = was_prefix;
                return key_table::KeyBindingEffect::client_selection_unchanged;
            }
            --remaining_routing_steps_;
            pending_bytes_.erase(0, decoded.byte_count);
            decoder_.consume();
            failed_action_consumed_ = true;
            return std::unexpected(binding.error());
        }
        if (*binding) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, decoded.byte_count);
            decoder_.consume();
            if (**binding == key_table::KeyBindingEffect::client_detached) {
                pending_bytes_.clear();
                return key_table::KeyBindingEffect::client_detached;
            }
            if (**binding == key_table::KeyBindingEffect::client_selection_changed ||
                **binding == key_table::KeyBindingEffect::pane_program_input_changed ||
                **binding == key_table::KeyBindingEffect::pane_mode_changed) {
                return **binding;
            }
            continue;
        }
        if (was_prefix || pane_table != root_table) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, decoded.byte_count);
            decoder_.consume();
            continue;
        }
        std::size_t passthrough_bytes = decoded.byte_count;
        const auto first = static_cast<unsigned char>(pending_bytes_.front());
        const bool plain_ascii = decoded.byte_count == 1 && first < 0x80U && first != 0x1bU;
        if (plain_ascii) {
            while (passthrough_bytes < pending_bytes_.size()) {
                const auto byte = static_cast<unsigned char>(pending_bytes_[passthrough_bytes]);
                const auto candidate = key_table::KeyCode{byte};
                if (byte >= 0x80U || byte == 0x1bU || prefix_keys.contains(candidate) ||
                    key_tables.contains(root_table, candidate)) {
                    break;
                }
                ++passthrough_bytes;
            }
        }
        if (!program_input_capacity) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, passthrough_bytes);
            decoder_.consume();
            continue;
        }
        const auto admitted_bytes =
            plain_ascii ? std::min(passthrough_bytes, *program_input_capacity) : passthrough_bytes;
        if (admitted_bytes == 0 || admitted_bytes > *program_input_capacity) {
            return key_table::KeyBindingEffect::client_selection_unchanged;
        }
        auto admitted = SendPaneInput::execute(
            server, {selection.pane, pending_bytes_.substr(0, admitted_bytes)});
        if (!admitted) {
            return std::unexpected(admitted.error());
        }
        --remaining_routing_steps_;
        pending_bytes_.erase(0, admitted_bytes);
        decoder_.consume();
        return key_table::KeyBindingEffect::pane_program_input_changed;
    }
    return key_table::KeyBindingEffect::client_selection_unchanged;
}
} // namespace tmux_cxx
