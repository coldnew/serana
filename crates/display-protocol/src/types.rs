use serde::{Deserialize, Serialize};

/// RGBA color with u8 components
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const RED: Self = Self::new(255, 0, 0);
    pub const GREEN: Self = Self::new(0, 255, 0);
    pub const BLUE: Self = Self::new(0, 0, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0);
    pub const CYAN: Self = Self::new(0, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255);
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

/// Text styling attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
    pub blink: bool,
}

impl Style {
    pub const fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            dim: false,
            reverse: false,
            blink: false,
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = Some(c);
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

    pub const fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub const fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub const fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub const fn blink(mut self) -> Self {
        self.blink = true;
        self
    }
}

/// A styled span of text
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
        }
    }

    pub fn width(&self) -> usize {
        self.text.chars().count()
    }
}

/// A line composed of styled spans
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

impl StyledLine {
    pub fn new(spans: Vec<StyledSpan>) -> Self {
        Self { spans }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            spans: vec![StyledSpan::plain(text)],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.spans.iter().all(|s| s.text.is_empty())
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.width()).sum()
    }
}
