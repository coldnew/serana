#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoraKeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl MoraKeyModifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        super_key: false,
    };

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

    pub const fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.super_key
    }

    pub const fn contains(&self, other: &Self) -> bool {
        (if self.ctrl { 1 } else { 0 } | if other.ctrl { 1 } else { 0 }) == (if self.ctrl { 1 } else { 0 })
            && (if self.alt { 1 } else { 0 } | if other.alt { 1 } else { 0 }) == (if self.alt { 1 } else { 0 })
            && (if self.shift { 1 } else { 0 } | if other.shift { 1 } else { 0 }) == (if self.shift { 1 } else { 0 })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoraKeyCode {
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
    F(u8),
    Insert,
    BackTab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoraKeyEvent {
    pub code: MoraKeyCode,
    pub modifiers: MoraKeyModifiers,
}

impl MoraKeyEvent {
    pub const fn new(code: MoraKeyCode, modifiers: MoraKeyModifiers) -> Self {
        Self { code, modifiers }
    }
}

impl From<crossterm::event::KeyEvent> for MoraKeyEvent {
    fn from(key: crossterm::event::KeyEvent) -> Self {
        let code = match key.code {
            crossterm::event::KeyCode::Char(c) => MoraKeyCode::Char(c),
            crossterm::event::KeyCode::Enter => MoraKeyCode::Enter,
            crossterm::event::KeyCode::Tab => MoraKeyCode::Tab,
            crossterm::event::KeyCode::Backspace => MoraKeyCode::Backspace,
            crossterm::event::KeyCode::Delete => MoraKeyCode::Delete,
            crossterm::event::KeyCode::Esc => MoraKeyCode::Esc,
            crossterm::event::KeyCode::Left => MoraKeyCode::Left,
            crossterm::event::KeyCode::Right => MoraKeyCode::Right,
            crossterm::event::KeyCode::Up => MoraKeyCode::Up,
            crossterm::event::KeyCode::Down => MoraKeyCode::Down,
            crossterm::event::KeyCode::Home => MoraKeyCode::Home,
            crossterm::event::KeyCode::End => MoraKeyCode::End,
            crossterm::event::KeyCode::PageUp => MoraKeyCode::PageUp,
            crossterm::event::KeyCode::PageDown => MoraKeyCode::PageDown,
            crossterm::event::KeyCode::F(n) => MoraKeyCode::F(n),
            crossterm::event::KeyCode::Insert => MoraKeyCode::Insert,
            crossterm::event::KeyCode::BackTab => MoraKeyCode::BackTab,
            _ => MoraKeyCode::Char('?'),
        };
        let modifiers = MoraKeyModifiers {
            ctrl: key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL),
            alt: key.modifiers.contains(crossterm::event::KeyModifiers::ALT),
            shift: key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT),
            super_key: key.modifiers.contains(crossterm::event::KeyModifiers::SUPER),
        };
        Self { code, modifiers }
    }
}

impl From<MoraKeyEvent> for crossterm::event::KeyEvent {
    fn from(key: MoraKeyEvent) -> Self {
        let code = match key.code {
            MoraKeyCode::Char(c) => crossterm::event::KeyCode::Char(c),
            MoraKeyCode::Enter => crossterm::event::KeyCode::Enter,
            MoraKeyCode::Tab => crossterm::event::KeyCode::Tab,
            MoraKeyCode::Backspace => crossterm::event::KeyCode::Backspace,
            MoraKeyCode::Delete => crossterm::event::KeyCode::Delete,
            MoraKeyCode::Esc => crossterm::event::KeyCode::Esc,
            MoraKeyCode::Left => crossterm::event::KeyCode::Left,
            MoraKeyCode::Right => crossterm::event::KeyCode::Right,
            MoraKeyCode::Up => crossterm::event::KeyCode::Up,
            MoraKeyCode::Down => crossterm::event::KeyCode::Down,
            MoraKeyCode::Home => crossterm::event::KeyCode::Home,
            MoraKeyCode::End => crossterm::event::KeyCode::End,
            MoraKeyCode::PageUp => crossterm::event::KeyCode::PageUp,
            MoraKeyCode::PageDown => crossterm::event::KeyCode::PageDown,
            MoraKeyCode::F(n) => crossterm::event::KeyCode::F(n),
            MoraKeyCode::Insert => crossterm::event::KeyCode::Insert,
            MoraKeyCode::BackTab => crossterm::event::KeyCode::BackTab,
        };
        let mut mods = crossterm::event::KeyModifiers::NONE;
        if key.modifiers.ctrl {
            mods |= crossterm::event::KeyModifiers::CONTROL;
        }
        if key.modifiers.alt {
            mods |= crossterm::event::KeyModifiers::ALT;
        }
        if key.modifiers.shift {
            mods |= crossterm::event::KeyModifiers::SHIFT;
        }
        if key.modifiers.super_key {
            mods |= crossterm::event::KeyModifiers::SUPER;
        }
        crossterm::event::KeyEvent::new(code, mods)
    }
}
