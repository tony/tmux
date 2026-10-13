#include "paste_buffer/register_key_bindings.hpp"
#include "paste_buffer/operations/paste_top_buffer_key_binding.hpp"

namespace tmux_cxx {
Result<void> register_paste_buffer_key_bindings(key_table::KeyBindingRegistry &registry) {
    return registry.add<PasteTopBufferKeyBinding>();
}
} // namespace tmux_cxx
