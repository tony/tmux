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

Result<key_table::KeyBindingEffect>
ClientKeyInput::route_pending(Server &server, ClientId client, ClientSelection selection,
                              const SessionPrefixKeys &prefix_keys,
                              std::optional<std::size_t> program_input_capacity,
                              const key_table::KeyTableCatalog &key_tables) {
    key_table::KeyBindingInvocation invocation{server, client, selection, prefix_keys.prefix};
    while (!pending_bytes_.empty()) {
        if (remaining_routing_steps_ == 0) {
            return key_table::KeyBindingEffect::client_selection_unchanged;
        }
        const auto key = key_table::KeyCode{
            static_cast<std::uint32_t>(static_cast<unsigned char>(pending_bytes_.front()))};
        if (active_table_ == root_table && prefix_keys.contains(key)) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, 1);
            active_table_ = prefix_table;
            continue;
        }

        const auto selected_table = active_table_;
        const bool selected_non_root_table = selected_table != root_table;
        if (selected_non_root_table) {
            active_table_ = root_table;
        }
        auto binding = key_tables.invoke(selected_table, key, invocation);
        if (!binding) {
            if (binding.error().code == TmuxErrorCode::backpressure) {
                active_table_ = selected_table;
                return key_table::KeyBindingEffect::client_selection_unchanged;
            }
            return std::unexpected(binding.error());
        }
        if (!*binding && selected_non_root_table) {
            binding = key_tables.invoke(root_table, key, invocation);
            if (!binding) {
                if (binding.error().code == TmuxErrorCode::backpressure) {
                    active_table_ = selected_table;
                    return key_table::KeyBindingEffect::client_selection_unchanged;
                }
                return std::unexpected(binding.error());
            }
        }
        if (*binding) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, 1);
            if (**binding == key_table::KeyBindingEffect::client_detached) {
                pending_bytes_.clear();
                return key_table::KeyBindingEffect::client_detached;
            }
            if (**binding == key_table::KeyBindingEffect::client_selection_changed ||
                **binding == key_table::KeyBindingEffect::pane_program_input_changed) {
                return **binding;
            }
            continue;
        }
        if (selected_non_root_table) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, 1);
            continue;
        }

        std::size_t passthrough_bytes = 0;
        while (passthrough_bytes < pending_bytes_.size()) {
            const auto candidate = key_table::KeyCode{static_cast<std::uint32_t>(
                static_cast<unsigned char>(pending_bytes_[passthrough_bytes]))};
            if (prefix_keys.contains(candidate) || key_tables.contains(root_table, candidate)) {
                break;
            }
            ++passthrough_bytes;
        }
        if (!program_input_capacity) {
            --remaining_routing_steps_;
            pending_bytes_.erase(0, passthrough_bytes);
            continue;
        }
        const auto admitted_bytes = std::min(passthrough_bytes, *program_input_capacity);
        if (admitted_bytes == 0) {
            return key_table::KeyBindingEffect::client_selection_unchanged;
        }
        auto admitted = SendPaneInput::execute(
            server, {selection.pane, pending_bytes_.substr(0, admitted_bytes)});
        if (!admitted) {
            return std::unexpected(admitted.error());
        }
        --remaining_routing_steps_;
        pending_bytes_.erase(0, admitted_bytes);
        *program_input_capacity -= admitted_bytes;
    }
    return key_table::KeyBindingEffect::client_selection_unchanged;
}
} // namespace tmux_cxx
