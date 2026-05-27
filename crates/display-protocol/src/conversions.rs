//! Conversions between display-protocol types and ratatui/crossterm types.

use crate::{Color, Style, StyledLine, KeyEvent, KeyCode, KeyModifiers};

// --- Color <-> ratatui::style::Color ---

impl From<Color> for ratatui::style::Color {
    fn from(c: Color) -> Self {
        ratatui::style::Color::Rgb(c.r, c.g, c.b)
    }
}

impl From<ratatui::style::Color> for Color {
    fn from(c: ratatui::style::Color) -> Self {
        match c {
            ratatui::style::Color::Rgb(r, g, b) => Color::new(r, g, b),
            ratatui::style::Color::Black => Color::new(0, 0, 0),
            ratatui::style::Color::Red => Color::new(205, 49, 49),
            ratatui::style::Color::Green => Color::new(13, 188, 121),
            ratatui::style::Color::Yellow => Color::new(229, 229, 16),
            ratatui::style::Color::Blue => Color::new(36, 114, 200),
            ratatui::style::Color::Magenta => Color::new(188, 63, 188),
            ratatui::style::Color::Cyan => Color::new(17, 168, 205),
            ratatui::style::Color::Gray => Color::new(229, 229, 229),
            ratatui::style::Color::DarkGray => Color::new(97, 97, 97),
            ratatui::style::Color::LightRed => Color::new(241, 76, 76),
            ratatui::style::Color::LightGreen => Color::new(35, 209, 139),
            ratatui::style::Color::LightYellow => Color::new(245, 245, 67),
            ratatui::style::Color::LightBlue => Color::new(72, 145, 215),
            ratatui::style::Color::LightMagenta => Color::new(214, 112, 214),
            ratatui::style::Color::LightCyan => Color::new(41, 184, 215),
            ratatui::style::Color::White => Color::new(255, 255, 255),
            _ => Color::new(229, 229, 229),
        }
    }
}

// --- Style <-> ratatui::style::Style ---

impl From<Style> for ratatui::style::Style {
    fn from(s: Style) -> Self {
        let mut style = ratatui::style::Style::default();
        if let Some(fg) = s.fg {
            style = style.fg(fg.into());
        }
        if let Some(bg) = s.bg {
            style = style.bg(bg.into());
        }
        let mut mods = ratatui::style::Modifier::empty();
        if s.bold {
            mods |= ratatui::style::Modifier::BOLD;
        }
        if s.italic {
            mods |= ratatui::style::Modifier::ITALIC;
        }
        if s.underline {
            mods |= ratatui::style::Modifier::UNDERLINED;
        }
        if s.dim {
            mods |= ratatui::style::Modifier::DIM;
        }
        if s.reverse {
            mods |= ratatui::style::Modifier::REVERSED;
        }
        if s.blink {
            mods |= ratatui::style::Modifier::SLOW_BLINK;
        }
        if s.strikethrough {
            mods |= ratatui::style::Modifier::CROSSED_OUT;
        }
        if !mods.is_empty() {
            style = style.add_modifier(mods);
        }
        style
    }
}

impl From<ratatui::style::Style> for Style {
    fn from(s: ratatui::style::Style) -> Self {
        Style {
            fg: s.fg.map(Color::from),
            bg: s.bg.map(Color::from),
            bold: s.add_modifier.contains(ratatui::style::Modifier::BOLD),
            italic: s.add_modifier.contains(ratatui::style::Modifier::ITALIC),
            underline: s.add_modifier.contains(ratatui::style::Modifier::UNDERLINED),
            strikethrough: s.add_modifier.contains(ratatui::style::Modifier::CROSSED_OUT),
            dim: s.add_modifier.contains(ratatui::style::Modifier::DIM),
            reverse: s.add_modifier.contains(ratatui::style::Modifier::REVERSED),
            blink: s.add_modifier.contains(ratatui::style::Modifier::SLOW_BLINK),
        }
    }
}

// --- StyledLine -> ratatui::text::Line ---

impl From<StyledLine> for ratatui::text::Line<'static> {
    fn from(line: StyledLine) -> Self {
        let spans: Vec<ratatui::text::Span<'static>> = line
            .spans
            .into_iter()
            .map(|s| ratatui::text::Span::styled(s.text, ratatui::style::Style::from(s.style)))
            .collect();
        ratatui::text::Line::from(spans)
    }
}

// --- KeyEvent <-> crossterm::event::KeyEvent ---

impl From<crossterm::event::KeyEvent> for KeyEvent {
    fn from(key: crossterm::event::KeyEvent) -> Self {
        let code = match key.code {
            crossterm::event::KeyCode::Char(c) => KeyCode::Char(c),
            crossterm::event::KeyCode::Enter => KeyCode::Enter,
            crossterm::event::KeyCode::Tab => KeyCode::Tab,
            crossterm::event::KeyCode::Backspace => KeyCode::Backspace,
            crossterm::event::KeyCode::Delete => KeyCode::Delete,
            crossterm::event::KeyCode::Esc => KeyCode::Esc,
            crossterm::event::KeyCode::Left => KeyCode::Left,
            crossterm::event::KeyCode::Right => KeyCode::Right,
            crossterm::event::KeyCode::Up => KeyCode::Up,
            crossterm::event::KeyCode::Down => KeyCode::Down,
            crossterm::event::KeyCode::Home => KeyCode::Home,
            crossterm::event::KeyCode::End => KeyCode::End,
            crossterm::event::KeyCode::PageUp => KeyCode::PageUp,
            crossterm::event::KeyCode::PageDown => KeyCode::PageDown,
            crossterm::event::KeyCode::F(n) => KeyCode::F(n),
            crossterm::event::KeyCode::Insert => KeyCode::Insert,
            crossterm::event::KeyCode::BackTab => KeyCode::BackTab,
            _ => KeyCode::Char('?'),
        };
        let modifiers = KeyModifiers {
            ctrl: key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL),
            alt: key.modifiers.contains(crossterm::event::KeyModifiers::ALT),
            shift: key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT),
            super_key: key.modifiers.contains(crossterm::event::KeyModifiers::SUPER),
        };
        Self { code, modifiers }
    }
}

impl From<KeyEvent> for crossterm::event::KeyEvent {
    fn from(key: KeyEvent) -> Self {
        let code = match key.code {
            KeyCode::Char(c) => crossterm::event::KeyCode::Char(c),
            KeyCode::Enter => crossterm::event::KeyCode::Enter,
            KeyCode::Tab => crossterm::event::KeyCode::Tab,
            KeyCode::Backspace => crossterm::event::KeyCode::Backspace,
            KeyCode::Delete => crossterm::event::KeyCode::Delete,
            KeyCode::Esc => crossterm::event::KeyCode::Esc,
            KeyCode::Left => crossterm::event::KeyCode::Left,
            KeyCode::Right => crossterm::event::KeyCode::Right,
            KeyCode::Up => crossterm::event::KeyCode::Up,
            KeyCode::Down => crossterm::event::KeyCode::Down,
            KeyCode::Home => crossterm::event::KeyCode::Home,
            KeyCode::End => crossterm::event::KeyCode::End,
            KeyCode::PageUp => crossterm::event::KeyCode::PageUp,
            KeyCode::PageDown => crossterm::event::KeyCode::PageDown,
            KeyCode::F(n) => crossterm::event::KeyCode::F(n),
            KeyCode::Insert => crossterm::event::KeyCode::Insert,
            KeyCode::BackTab => crossterm::event::KeyCode::BackTab,
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
