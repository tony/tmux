use crate::window::Window;

pub struct Session {
    pub name: String,
    pub windows: Vec<Window>,
    pub active_window_idx: usize,
}

impl Session {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            windows: Vec::new(),
            active_window_idx: 0,
        }
    }

    pub fn add_window(&mut self, window: Window) {
        self.windows.push(window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mux_types::Size;

    #[test]
    fn test_session_creation() {
        let s = Session::new("test");
        assert_eq!(s.name, "test");
        assert!(s.windows.is_empty());
        assert_eq!(s.active_window_idx, 0);
    }

    #[test]
    fn test_add_window() {
        let mut s = Session::new("test");
        let w = Window::new(1, "win1", Size { width: 80, height: 24 });
        s.add_window(w);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].name, "win1");
    }

    #[test]
    fn test_session_default_index() {
        let s = Session::new("foo");
        assert_eq!(s.active_window_idx, 0);
    }
}
