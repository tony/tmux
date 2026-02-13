//! # mux-types
//!
//! Core types for the TermForge terminal multiplexer.
//! L0 crate with minimal dependencies.
//!
//! ## Key Types
//! - [`Cell`]: A terminal cell with grapheme, width, flags, attributes, colours.
//! - [`CellFlags`]: Bitflags matching tmux's `GRID_FLAG_*` values (INV-119).
//! - [`Colour`]: Terminal colour (indexed, RGB, default).
//! - [`Attrs`]: Text attributes (bold, italic, underline, etc.).
//! - [`KeyCode`]: Terminal key codes for input handling.
//! - [`MouseEvent`]: Mouse button/motion/scroll events.
//! - Entity IDs: [`SessionId`], [`WindowId`], [`PaneId`], [`ClientId`].

#![forbid(unsafe_code)]

use bitflags::bitflags;
use mux_grapheme_arena::GraphemeId;

// ---------------------------------------------------------------------------
// Cell flags (matching tmux GRID_FLAG_* from tmux.h)
// ---------------------------------------------------------------------------

bitflags! {
    /// Cell flags matching tmux's `GRID_FLAG_*` values.
    ///
    /// Note: `GRID_FLAG_FG256` (0x01), `GRID_FLAG_BG256` (0x02), and
    /// `GRID_FLAG_NOPALETTE` (0x20) are tmux's compact encoding flags --
    /// unnecessary in TermForge because the [`Colour`] enum handles
    /// representation directly.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CellFlags: u8 {
        /// Wide character padding cell (tmux GRID_FLAG_PADDING = 0x04, tmux.h:744).
        /// 42 occurrences across 9 files in tmux source.
        const PADDING  = 0x04;
        /// Extended grapheme cluster (tmux GRID_FLAG_EXTENDED = 0x08, tmux.h:745).
        const EXTENDED = 0x08;
        /// Cell is selected (tmux GRID_FLAG_SELECTED = 0x10, tmux.h:746).
        const SELECTED = 0x10;
        /// Cell has been explicitly cleared (tmux GRID_FLAG_CLEARED = 0x40, tmux.h:748).
        const CLEARED  = 0x40;
        /// Cell contains a tab (tmux GRID_FLAG_TAB = 0x80, tmux.h:749).
        const TAB      = 0x80;
    }
}

impl Default for CellFlags {
    fn default() -> Self {
        Self::empty()
    }
}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// Terminal colour representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Colour {
    /// Default terminal foreground/background.
    Default,
    /// 256-colour palette index (0-255).
    Indexed(u8),
    /// True-colour RGB.
    Rgb { r: u8, g: u8, b: u8 },
}

impl Default for Colour {
    fn default() -> Self {
        Self::Default
    }
}

// ---------------------------------------------------------------------------
// Text attributes
// ---------------------------------------------------------------------------

bitflags! {
    /// Text attributes for terminal cells.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Attrs: u16 {
        const BOLD          = 0x0001;
        const DIM           = 0x0002;
        const ITALIC        = 0x0004;
        const UNDERSCORE    = 0x0008;
        const BLINK         = 0x0010;
        const REVERSE       = 0x0020;
        const HIDDEN        = 0x0040;
        const STRIKETHROUGH = 0x0080;
        const DOUBLE_UNDER  = 0x0100;
        const CURLY_UNDER   = 0x0200;
        const DOTTED_UNDER  = 0x0400;
        const DASHED_UNDER  = 0x0800;
        const OVERLINE      = 0x1000;
    }
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A single terminal cell.
///
/// Holds a grapheme reference, display width, flags, attributes, and colours.
/// This matches the v25 DEFINITIVE cell model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Arena-allocated UTF-8 grapheme cluster.
    pub grapheme: GraphemeId,
    /// Display width: 0=padding, 1=normal, 2=CJK wide.
    pub width: u8,
    /// Cell flags matching tmux GRID_FLAG_*.
    pub flags: CellFlags,
    /// Text attributes (bold, italic, etc.).
    pub attrs: Attrs,
    /// Foreground colour.
    pub fg: Colour,
    /// Background colour.
    pub bg: Colour,
    /// Underline colour (for colored underlines).
    pub us: Colour,
    /// Hyperlink ID (OSC 8). 0 = no hyperlink.
    pub link: u32,
}

impl Cell {
    /// Create a default (empty) cell.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            grapheme: GraphemeId::Empty,
            width: 1,
            flags: CellFlags::empty(),
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
            link: 0,
        }
    }

    /// Create a padding cell for wide characters.
    #[must_use]
    pub const fn padding() -> Self {
        Self {
            grapheme: GraphemeId::Empty,
            width: 0,
            flags: CellFlags::PADDING,
            attrs: Attrs::empty(),
            fg: Colour::Default,
            bg: Colour::Default,
            us: Colour::Default,
            link: 0,
        }
    }

    /// Whether this cell is a padding cell for a wide character.
    #[must_use]
    pub const fn is_padding(&self) -> bool {
        self.flags.contains(CellFlags::PADDING)
    }

    /// Whether this cell is empty (default grapheme, no flags).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self.grapheme, GraphemeId::Empty) && self.flags.is_empty()
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::empty()
    }
}

// ---------------------------------------------------------------------------
// Key codes
// ---------------------------------------------------------------------------

/// Terminal key codes for input handling.
///
/// Covers function keys, cursor keys, keypad, and modified keys.
/// Parsing raw terminal byte sequences into `KeyCode` is done by mux-parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A unicode character (possibly modified).
    Char(char),
    /// Function key F1-F12.
    F(u8),
    /// Cursor up.
    Up,
    /// Cursor down.
    Down,
    /// Cursor left.
    Left,
    /// Cursor right.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Insert key.
    Insert,
    /// Delete key.
    Delete,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Backspace.
    Backspace,
    /// Tab.
    Tab,
    /// Backtab (Shift+Tab).
    BackTab,
    /// Enter / Return.
    Enter,
    /// Escape.
    Escape,
}

/// Key modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyModifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub meta: bool,
}

/// A key event with code and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

// ---------------------------------------------------------------------------
// Mouse events
// ---------------------------------------------------------------------------

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    /// Extended buttons (button 4+).
    Extended(u8),
}

/// Mouse event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseEventKind {
    Press(MouseButton),
    Release(MouseButton),
    Drag(MouseButton),
    Motion,
}

/// A mouse event with position and modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseEvent {
    pub kind: MouseEventKind,
    pub x: u16,
    pub y: u16,
    pub modifiers: KeyModifiers,
}

// ---------------------------------------------------------------------------
// Entity IDs (slotmap keys)
// ---------------------------------------------------------------------------

/// Strongly typed session identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub u64);

/// Strongly typed window identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Strongly typed pane identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

/// Strongly typed client identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

// ---------------------------------------------------------------------------
// Size
// ---------------------------------------------------------------------------

/// Terminal size in columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    pub cols: u32,
    pub rows: u32,
}

impl Size {
    /// Create a new size.
    #[must_use]
    pub const fn new(cols: u32, rows: u32) -> Self {
        Self { cols, rows }
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Core error type for TermForge operations.
#[derive(Debug, thiserror::Error)]
pub enum TermForgeError {
    #[error("PTY error: {0}")]
    Pty(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("kernel error: {0}")]
    Kernel(String),
    #[error("grid error: {0}")]
    Grid(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("target not found: {0}")]
    TargetNotFound(String),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("window not found: {0}")]
    WindowNotFound(String),
    #[error("pane not found: {0}")]
    PaneNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("channel closed")]
    ChannelClosed,
    #[error("timeout")]
    Timeout,
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not supported: {0}")]
    NotSupported(String),
    #[error("capacity exceeded: {0}")]
    CapacityExceeded(String),
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience result type.
pub type Result<T> = std::result::Result<T, TermForgeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_flags_padding_matches_tmux() {
        assert_eq!(CellFlags::PADDING.bits(), 0x04);
    }

    #[test]
    fn cell_flags_extended_matches_tmux() {
        assert_eq!(CellFlags::EXTENDED.bits(), 0x08);
    }

    #[test]
    fn cell_flags_selected_matches_tmux() {
        assert_eq!(CellFlags::SELECTED.bits(), 0x10);
    }

    #[test]
    fn cell_flags_cleared_matches_tmux() {
        assert_eq!(CellFlags::CLEARED.bits(), 0x40);
    }

    #[test]
    fn cell_flags_tab_matches_tmux() {
        assert_eq!(CellFlags::TAB.bits(), 0x80);
    }

    #[test]
    fn empty_cell_defaults() {
        let cell = Cell::empty();
        assert!(cell.is_empty());
        assert!(!cell.is_padding());
        assert_eq!(cell.width, 1);
    }

    #[test]
    fn padding_cell() {
        let cell = Cell::padding();
        assert!(cell.is_padding());
        assert_eq!(cell.width, 0);
    }

    #[test]
    fn size_constructor() {
        let s = Size::new(80, 24);
        assert_eq!(s.cols, 80);
        assert_eq!(s.rows, 24);
    }
}
