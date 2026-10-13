#include "paste_buffer/operations/paste_top_buffer_key_binding.hpp"

namespace tmux_cxx {
Result<key_table::KeyBindingEffect>
PasteTopBufferKeyBinding::execute(key_table::KeyBindingInvocation &invocation) {
    auto result = PasteTopBuffer::execute(invocation.server, {invocation.selection.pane});
    if (!result) {
        return std::unexpected(result.error());
    }
    return key_table::KeyBindingEffect::pane_program_input_changed;
}
} // namespace tmux_cxx
