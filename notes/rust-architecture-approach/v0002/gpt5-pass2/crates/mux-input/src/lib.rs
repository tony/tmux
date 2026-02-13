#![forbid(unsafe_code)]

use mux_types::PaneId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    Key(String),
    Mouse { row: u16, col: u16, button: u8 },
    Paste(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedInput {
    pub pane: PaneId,
    pub event: InputEvent,
}

#[must_use]
pub fn route_to_active(active: PaneId, event: InputEvent) -> RoutedInput {
    RoutedInput { pane: active, event }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_key_to_active_pane() {
        let out = route_to_active(PaneId(8), InputEvent::Key("j".into()));
        assert_eq!(out.pane, PaneId(8));
    }

    #[test]
    fn routes_mouse_to_active_pane() {
        let out = route_to_active(PaneId(1), InputEvent::Mouse { row: 2, col: 3, button: 1 });
        assert_eq!(out.pane, PaneId(1));
    }

    #[test]
    fn routes_paste() {
        let out = route_to_active(PaneId(2), InputEvent::Paste("abc".into()));
        assert!(matches!(out.event, InputEvent::Paste(_)));
    }

    #[test]
    fn routed_input_is_eq() {
        let a = route_to_active(PaneId(2), InputEvent::Key("x".into()));
        let b = route_to_active(PaneId(2), InputEvent::Key("x".into()));
        assert_eq!(a, b);
    }
}
