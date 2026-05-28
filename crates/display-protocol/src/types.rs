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
        Self::BLACK
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
    pub underline_color: Option<Color>,
}

impl Style {
    pub fn fg(mut self, c: Color) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = Some(c);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    pub fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    pub fn blink(mut self) -> Self {
        self.blink = true;
        self
    }

    pub fn underline_color(mut self, c: Color) -> Self {
        self.underline_color = Some(c);
        self
    }

    pub fn merge(&self, other: &Style) -> Style {
        Style {
            fg: other.fg.or(self.fg),
            bg: other.bg.or(self.bg),
            bold: self.bold || other.bold,
            italic: self.italic || other.italic,
            underline: self.underline || other.underline,
            strikethrough: self.strikethrough || other.strikethrough,
            dim: self.dim || other.dim,
            reverse: self.reverse || other.reverse,
            blink: self.blink || other.blink,
            underline_color: other.underline_color.or(self.underline_color),
        }
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

    /// Get the character at a column index (0-based), crossing span boundaries.
    pub fn char_at(&self, col: usize) -> Option<char> {
        let mut offset = 0;
        for span in &self.spans {
            let span_len = span.text.chars().count();
            if col < offset + span_len {
                return span.text.chars().nth(col - offset);
            }
            offset += span_len;
        }
        None
    }

    /// Extract a character-range substring as a new StyledLine.
    /// `start` is 0-based character offset, `len` is max characters to keep.
    pub fn substr(&self, start: usize, len: usize) -> StyledLine {
        let mut spans = Vec::new();
        let mut offset = 0;
        for span in &self.spans {
            let span_len = span.text.chars().count();
            let span_end = offset + span_len;
            if span_end <= start {
                offset = span_end;
                continue;
            }
            if offset >= start + len {
                break;
            }
            let skip = start.saturating_sub(offset);
            let take = (len + start).saturating_sub(offset).min(span_len) - skip;
            let text: String = span.text.chars().skip(skip).take(take).collect();
            if !text.is_empty() {
                spans.push(StyledSpan {
                    text,
                    style: span.style,
                });
            }
            offset = span_end;
        }
        StyledLine { spans }
    }
}
