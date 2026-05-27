use serde::{Deserialize, Serialize};

/// Key code for keyboard events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    BackTab,
    F(u8),
}

/// Key modifier state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    pub const EMPTY: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        super_key: false,
    };

    pub const NONE: Self = Self::EMPTY;

    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
        super_key: false,
    };

    pub const CONTROL: Self = Self::CTRL;

    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
        super_key: false,
    };

    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        super_key: false,
    };

    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.super_key
    }

    pub fn contains(&self, other: &Self) -> bool {
        (if self.ctrl { 1 } else { 0 } | if other.ctrl { 1 } else { 0 }) == (if self.ctrl { 1 } else { 0 })
            && (if self.alt { 1 } else { 0 } | if other.alt { 1 } else { 0 }) == (if self.alt { 1 } else { 0 })
            && (if self.shift { 1 } else { 0 } | if other.shift { 1 } else { 0 }) == (if self.shift { 1 } else { 0 })
    }
}

/// A keyboard event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn char(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::EMPTY,
        }
    }
}

/// Mouse event kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseEventKind {
    Press,
    Release,
    Drag,
    ScrollUp,
    ScrollDown,
}

/// Input events from frontend to core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    /// Keyboard input
    Key(KeyEvent),
    /// Terminal/window resize
    Resize { width: u16, height: u16 },
    /// Mouse input
    Mouse {
        x: u16,
        y: u16,
        kind: MouseEventKind,
        modifiers: KeyModifiers,
    },
    /// Window gained focus
    FocusGained,
    /// Window lost focus
    FocusLost,
    /// Paste text
    Paste(String),
}
