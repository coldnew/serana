//! ANSI styling primitives.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Rgb(u8, u8, u8),
}

impl Color {
    /// Returns the ANSI foreground escape sequence.
    pub fn fg_ansi(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::BrightBlack => "\x1b[90m",
            Color::BrightRed => "\x1b[91m",
            Color::BrightGreen => "\x1b[92m",
            Color::BrightYellow => "\x1b[93m",
            Color::BrightBlue => "\x1b[94m",
            Color::BrightMagenta => "\x1b[95m",
            Color::BrightCyan => "\x1b[96m",
            Color::BrightWhite => "\x1b[97m",
            _ => "",
        }
    }

    /// Returns the ANSI background escape sequence.
    pub fn bg_ansi(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[40m",
            Color::Red => "\x1b[41m",
            Color::Green => "\x1b[42m",
            Color::Yellow => "\x1b[43m",
            Color::Blue => "\x1b[44m",
            Color::Magenta => "\x1b[45m",
            Color::Cyan => "\x1b[46m",
            Color::White => "\x1b[47m",
            Color::BrightBlack => "\x1b[100m",
            Color::BrightRed => "\x1b[101m",
            Color::BrightGreen => "\x1b[102m",
            Color::BrightYellow => "\x1b[103m",
            Color::BrightBlue => "\x1b[104m",
            Color::BrightMagenta => "\x1b[105m",
            Color::BrightCyan => "\x1b[106m",
            Color::BrightWhite => "\x1b[107m",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
}

impl Style {
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            dim: false,
        }
    }

    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// Wrap `text` with ANSI codes for this style.
    pub fn apply(&self, text: &str) -> String {
        if self.fg.is_none() && self.bg.is_none() && !self.bold && !self.italic && !self.dim {
            return text.to_owned();
        }
        let mut out = String::with_capacity(text.len() + 32);
        if let Some(fg) = &self.fg {
            out.push_str(fg.fg_ansi());
        }
        if let Some(bg) = &self.bg {
            match bg {
                Color::Rgb(r, g, b) => {
                    use std::fmt::Write;
                    let _ = write!(out, "\x1b[48;2;{};{};{}m", r, g, b);
                }
                other => out.push_str(other.bg_ansi()),
            }
        }
        if self.bold {
            out.push_str("\x1b[1m");
        }
        if self.dim {
            out.push_str("\x1b[2m");
        }
        if self.italic {
            out.push_str("\x1b[3m");
        }
        out.push_str(text);
        out.push_str("\x1b[0m");
        out
    }
}

/// Shorthand color constants (SCREAMING_CASE per Rust const convention).
pub mod Colors {
    use super::Color;
    pub const BLACK: Color = Color::Black;
    pub const RED: Color = Color::Red;
    pub const GREEN: Color = Color::Green;
    pub const YELLOW: Color = Color::Yellow;
    pub const BLUE: Color = Color::Blue;
    pub const MAGENTA: Color = Color::Magenta;
    pub const CYAN: Color = Color::Cyan;
    pub const WHITE: Color = Color::White;
    pub const GRAY: Color = Color::BrightBlack;
    pub const BRIGHT_RED: Color = Color::BrightRed;
    pub const BRIGHT_GREEN: Color = Color::BrightGreen;
    pub const BRIGHT_YELLOW: Color = Color::BrightYellow;
    pub const BRIGHT_BLUE: Color = Color::BrightBlue;
    pub const BRIGHT_MAGENTA: Color = Color::BrightMagenta;
    pub const BRIGHT_CYAN: Color = Color::BrightCyan;
    pub const BRIGHT_WHITE: Color = Color::BrightWhite;
}
