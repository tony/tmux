#include "pane/register_key_bindings.hpp"

#include "pane/operations/copy_mode_key_bindings.hpp"
#include "pane/operations/select_next_pane_key_binding.hpp"
#include "pane/operations/send_prefix_key_binding.hpp"
#include "pane/operations/split_pane_key_bindings.hpp"

namespace tmux_cxx {
namespace {
using CursorLeft =
    CopyCursorCommand<CopyCursorDirection::left>; ///< @copybrief tmux_cxx::CopyCursorCommand
using CursorRight =
    CopyCursorCommand<CopyCursorDirection::right>; ///< @copybrief tmux_cxx::CopyCursorCommand
using CursorUp =
    CopyCursorCommand<CopyCursorDirection::up>; ///< @copybrief tmux_cxx::CopyCursorCommand
using CursorDown =
    CopyCursorCommand<CopyCursorDirection::down>; ///< @copybrief tmux_cxx::CopyCursorCommand
using LineStart = CopyCursorDestinationCommand<
    CopyCursorDestination::line_start>; ///< @copybrief tmux_cxx::CopyCursorDestinationCommand
using LineEnd = CopyCursorDestinationCommand<
    CopyCursorDestination::line_end>; ///< @copybrief tmux_cxx::CopyCursorDestinationCommand
using HistoryTop = CopyCursorDestinationCommand<
    CopyCursorDestination::history_start>; ///< @copybrief tmux_cxx::CopyCursorDestinationCommand
using HistoryBottom = CopyCursorDestinationCommand<
    CopyCursorDestination::history_end>; ///< @copybrief tmux_cxx::CopyCursorDestinationCommand
using ScrollUp =
    CopyScrollCommand<CopyScrollMotion::line_up>; ///< @copybrief tmux_cxx::CopyScrollCommand
using ScrollDown =
    CopyScrollCommand<CopyScrollMotion::line_down>; ///< @copybrief tmux_cxx::CopyScrollCommand
using HalfPageUp =
    CopyScrollCommand<CopyScrollMotion::half_page_up>; ///< @copybrief tmux_cxx::CopyScrollCommand
using HalfPageDown =
    CopyScrollCommand<CopyScrollMotion::half_page_down>; ///< @copybrief tmux_cxx::CopyScrollCommand
using PageUp =
    CopyScrollCommand<CopyScrollMotion::page_up>; ///< @copybrief tmux_cxx::CopyScrollCommand
using PageDown =
    CopyScrollCommand<CopyScrollMotion::page_down>; ///< @copybrief tmux_cxx::CopyScrollCommand

/** @brief Bind one key in the persistent emacs copy table to its typed native action. */
template <key_table::KeyCode Key, class Command>
using EmacsCopyKey = CopyModeKeyBinding<WindowModeKeys::emacs, Key, Command>;
/** @brief Bind one key in the persistent vi copy table to its typed native action. */
template <key_table::KeyCode Key, class Command>
using ViCopyKey = CopyModeKeyBinding<WindowModeKeys::vi, Key, Command>;

/** @brief Register typed copy bindings in declaration order, stopping at the first startup failure.
 */
template <class... Bindings>
Result<void> register_copy_mode_bindings(key_table::KeyBindingRegistry &registry) {
    Result<void> registration;
    /** @brief Preserve the first failure instead of extending an invalid startup registry. */
    const auto add = [&]<class Binding>() {
        if (registration) {
            registration = registry.add<Binding>();
        }
    };
    (add.template operator()<Bindings>(), ...);
    return registration;
}
} // namespace

Result<void> register_pane_key_bindings(key_table::KeyBindingRegistry &registry) {
    if (auto registration = registry.add<SendPrefixKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SplitPaneTopBottomKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SplitPaneLeftRightKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<SelectNextPaneKeyBinding>(); !registration) {
        return registration;
    }
    if (auto registration = registry.add<EnterCopyModeKeyBinding>(); !registration) {
        return registration;
    }
    using namespace key_table;
    return register_copy_mode_bindings<
        EmacsCopyKey<up_key, CursorUp>, EmacsCopyKey<down_key, CursorDown>,
        EmacsCopyKey<left_key, CursorLeft>, EmacsCopyKey<right_key, CursorRight>,
        EmacsCopyKey<home_key, LineStart>, EmacsCopyKey<end_key, LineEnd>,
        EmacsCopyKey<page_up_key, PageUp>, EmacsCopyKey<page_down_key, PageDown>,
        EmacsCopyKey<KeyCode{up_key.value, KeyModifier::control}, ScrollUp>,
        EmacsCopyKey<KeyCode{down_key.value, KeyModifier::control}, ScrollDown>,
        EmacsCopyKey<KeyCode{up_key.value, KeyModifier::meta}, HalfPageUp>,
        EmacsCopyKey<KeyCode{down_key.value, KeyModifier::meta}, HalfPageDown>,
        EmacsCopyKey<control_key('a'), LineStart>, EmacsCopyKey<control_key('e'), LineEnd>,
        EmacsCopyKey<control_key('b'), CursorLeft>, EmacsCopyKey<control_key('f'), CursorRight>,
        EmacsCopyKey<control_key('p'), CursorUp>, EmacsCopyKey<control_key('n'), CursorDown>,
        EmacsCopyKey<control_key('v'), PageDown>, EmacsCopyKey<KeyCode{' '}, PageDown>,
        EmacsCopyKey<KeyCode{'v', KeyModifier::meta}, PageUp>,
        EmacsCopyKey<KeyCode{'<', KeyModifier::meta}, HistoryTop>,
        EmacsCopyKey<KeyCode{'>', KeyModifier::meta}, HistoryBottom>,
        EmacsCopyKey<KeyCode{0}, BeginCopySelectionCommand>,
        EmacsCopyKey<control_key('g'), ClearCopySelectionCommand>,
        CancelCopyModeKeyBinding<WindowModeKeys::emacs, KeyCode{'q'}>,
        CancelCopyModeKeyBinding<WindowModeKeys::emacs, escape_key>,
        CancelCopyModeKeyBinding<WindowModeKeys::emacs, control_key('c')>,
        CopySelectionAndCancelKeyBinding<WindowModeKeys::emacs, control_key('w')>,
        CopySelectionAndCancelKeyBinding<WindowModeKeys::emacs, KeyCode{'w', KeyModifier::meta}>,
        ViCopyKey<up_key, CursorUp>, ViCopyKey<down_key, CursorDown>,
        ViCopyKey<left_key, CursorLeft>, ViCopyKey<right_key, CursorRight>,
        ViCopyKey<home_key, LineStart>, ViCopyKey<end_key, LineEnd>, ViCopyKey<page_up_key, PageUp>,
        ViCopyKey<page_down_key, PageDown>,
        ViCopyKey<KeyCode{up_key.value, KeyModifier::control}, ScrollUp>,
        ViCopyKey<KeyCode{down_key.value, KeyModifier::control}, ScrollDown>,
        ViCopyKey<KeyCode{'h'}, CursorLeft>, ViCopyKey<KeyCode{'l'}, CursorRight>,
        ViCopyKey<KeyCode{'k'}, CursorUp>, ViCopyKey<KeyCode{'j'}, CursorDown>,
        ViCopyKey<control_key('h'), CursorLeft>, ViCopyKey<KeyCode{127}, CursorLeft>,
        ViCopyKey<KeyCode{'0'}, LineStart>, ViCopyKey<KeyCode{'$'}, LineEnd>,
        ViCopyKey<KeyCode{'g'}, HistoryTop>, ViCopyKey<KeyCode{'G'}, HistoryBottom>,
        ViCopyKey<KeyCode{'K'}, ScrollUp>, ViCopyKey<KeyCode{'J'}, ScrollDown>,
        ViCopyKey<control_key('y'), ScrollUp>, ViCopyKey<control_key('e'), ScrollDown>,
        ViCopyKey<control_key('u'), HalfPageUp>, ViCopyKey<control_key('d'), HalfPageDown>,
        ViCopyKey<control_key('b'), PageUp>, ViCopyKey<control_key('f'), PageDown>,
        ViCopyKey<KeyCode{' '}, BeginCopySelectionCommand>,
        ViCopyKey<escape_key, ClearCopySelectionCommand>,
        CancelCopyModeKeyBinding<WindowModeKeys::vi, KeyCode{'q'}>,
        CancelCopyModeKeyBinding<WindowModeKeys::vi, control_key('c')>,
        CopySelectionAndCancelKeyBinding<WindowModeKeys::vi, KeyCode{13}>,
        CopySelectionAndCancelKeyBinding<WindowModeKeys::vi, control_key('j')>>(registry);
}
} // namespace tmux_cxx
