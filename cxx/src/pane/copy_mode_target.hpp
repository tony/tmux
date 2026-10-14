#pragma once

#include "pane/pane.hpp"
#include "protocol/native/framing.hpp"

#include <map>
#include <memory>
#include <stdexcept>

namespace tmux_cxx {
/** @brief Borrow a pane target during serialized execution; zero and deleted identities fail. */
inline Result<Pane *> copy_mode_pane(std::map<PaneId, std::unique_ptr<Pane>> &panes, PaneId id) {
    if (id.value == 0) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "pane identity must be nonzero"});
    }
    const auto entry = panes.find(id);
    if (entry == panes.end()) {
        return std::unexpected(TmuxError{TmuxErrorCode::not_found, "pane no longer exists"});
    }
    return entry->second.get();
}
/** @brief Borrow the active pane mode until a serialized mutation replaces or cancels it. */
inline Result<CopyMode *> active_copy_mode(Pane &pane) {
    if (!pane.copy_mode) {
        return std::unexpected(
            TmuxError{TmuxErrorCode::invalid_argument, "pane is not in copy mode"});
    }
    return &*pane.copy_mode;
}
/** @brief Reject unknown copy-choice wire representations before invoking their typed operation. */
template <class Choice>
Choice read_copy_mode_choice(protocol::native::NativePayloadReader &reader, bool (*valid)(Choice)) {
    const auto choice = static_cast<Choice>(reader.u32());
    if (!valid(choice)) {
        throw std::invalid_argument("invalid copy-mode choice");
    }
    return choice;
}
} // namespace tmux_cxx
