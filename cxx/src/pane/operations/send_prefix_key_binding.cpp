#include "pane/operations/send_prefix_key_binding.hpp"

#include <string>

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
SendPrefixKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    if (!invocation.prefix) {
        return key_table::KeyBindingEffect::client_selection_unchanged;
    }
    if (invocation.prefix->value > 0xffU) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::unsupported, "configured prefix has no single-byte encoding"});
    }
    const auto prefix = static_cast<char>(invocation.prefix->value);
    if (auto sent = SendPaneInput::execute(invocation.server,
                                           {invocation.selection.pane, std::string(1, prefix)});
        !sent) {
        return std::unexpected(sent.error());
    }
    return key_table::KeyBindingEffect::pane_program_input_changed;
}
} // namespace tmux_cxx
