#include "register_operations.hpp"

#include <tmux_cxx/paste_buffer/operations/add_paste_buffer.hpp>
#include <tmux_cxx/paste_buffer/operations/delete_paste_buffer.hpp>
#include <tmux_cxx/paste_buffer/operations/list_paste_buffers.hpp>
#include <tmux_cxx/paste_buffer/operations/paste_buffer_into_pane.hpp>
#include <tmux_cxx/paste_buffer/operations/paste_top_buffer.hpp>
#include <tmux_cxx/paste_buffer/operations/read_paste_buffer.hpp>
#include <tmux_cxx/paste_buffer/operations/set_paste_buffer.hpp>

namespace tmux_cxx {
Result<void> register_paste_buffer_operations(protocol::native::OperationRegistry &registry) {
    if (auto registration = registry.add<AddPasteBuffer>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ListPasteBuffers>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<PasteTopBuffer>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SetPasteBuffer>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<ReadPasteBuffer>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<DeletePasteBuffer>(); !registration) {
        return registration;
    }
    return registry.add<PasteBufferIntoPane>();
}
} // namespace tmux_cxx
