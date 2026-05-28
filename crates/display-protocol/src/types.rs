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

/// Selection mode for text and list selections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum SelectionMode {
    /// Character-level selection (most text editors).
    #[default]
    Char,
    /// Line-level selection (vim linewise).
    Line,
    /// Rectangular/block selection (column edit).
    Block,
}

/// A text selection with anchor and head positions.
///
/// Anchor is where the selection started, head is where it currently ends.
/// This model supports both cursor-based text selection and list/tree selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Where the selection started (line, col).
    pub anchor_line: u32,
    pub anchor_col: u32,
    /// Where the selection head currently is (line, col).
    pub head_line: u32,
    pub head_col: u32,
    /// Selection granularity.
    pub mode: SelectionMode,
}

impl Selection {
    /// Create a cursor (collapsed selection at a single point).
    pub fn cursor(line: u32, col: u32) -> Self {
        Self {
            anchor_line: line,
            anchor_col: col,
            head_line: line,
            head_col: col,
            mode: SelectionMode::Char,
        }
    }

    /// Create a selection between two points.
    pub fn range(
        anchor_line: u32, anchor_col: u32,
        head_line: u32, head_col: u32,
        mode: SelectionMode,
    ) -> Self {
        Self { anchor_line, anchor_col, head_line, head_col, mode }
    }

    /// Whether the selection is collapsed (just a cursor, no range).
    pub fn is_collapsed(&self) -> bool {
        self.anchor_line == self.head_line && self.anchor_col == self.head_col
    }

    /// Whether a given line is covered by this selection.
    pub fn contains_line(&self, line: u32) -> bool {
        if self.is_collapsed() { return false; }
        let (start, end) = self.line_range();
        line >= start && line <= end
    }

    /// Whether a given (line, col) position is within the selection.
    pub fn contains(&self, line: u32, col: u32) -> bool {
        if self.is_collapsed() { return false; }
        let (start_line, start_col, end_line, end_col) = self.normalized_range();
        match self.mode {
            SelectionMode::Char => {
                if line < start_line || line > end_line { return false; }
                if line == start_line && col < start_col { return false; }
                if line == end_line && col >= end_col { return false; }
                true
            }
            SelectionMode::Line => {
                line >= start_line && line <= end_line
            }
            SelectionMode::Block => {
                let min_col = start_col.min(end_col);
                let max_col = start_col.max(end_col);
                line >= start_line && line <= end_line && col >= min_col && col < max_col
            }
        }
    }

    /// Get (start_line, end_line) after normalizing anchor/head.
    pub fn line_range(&self) -> (u32, u32) {
        if self.anchor_line <= self.head_line {
            (self.anchor_line, self.head_line)
        } else {
            (self.head_line, self.anchor_line)
        }
    }

    /// Get (start_line, start_col, end_line, end_col) normalized (anchor ≤ head).
    pub fn normalized_range(&self) -> (u32, u32, u32, u32) {
        if self.anchor_line < self.head_line
            || (self.anchor_line == self.head_line && self.anchor_col <= self.head_col)
        {
            (self.anchor_line, self.anchor_col, self.head_line, self.head_col)
        } else {
            (self.head_line, self.head_col, self.anchor_line, self.anchor_col)
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::cursor(0, 0)
    }
}

/// Multi-selection support (e.g. multiple cursors in VSCode).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MultiSelection {
    /// Primary (last added) selection.
    pub primary: Selection,
    /// Additional secondary selections.
    pub secondary: Vec<Selection>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_is_collapsed() {
        let sel = Selection::cursor(5, 10);
        assert!(sel.is_collapsed());
        assert_eq!(sel.head_line, 5);
        assert_eq!(sel.head_col, 10);
    }

    #[test]
    fn test_selection_contains_char() {
        // Select from (1,3) to (2,5)
        let sel = Selection::range(1, 3, 2, 5, SelectionMode::Char);
        assert!(!sel.is_collapsed());
        assert!(sel.contains(1, 3));  // start inclusive
        assert!(sel.contains(1, 10)); // within first line
        assert!(sel.contains(2, 4));  // within last line
        assert!(!sel.contains(2, 5)); // end exclusive
        assert!(!sel.contains(0, 0)); // before
        assert!(!sel.contains(3, 0)); // after
    }

    #[test]
    fn test_selection_reversed() {
        // Head before anchor (user selected backwards)
        let sel = Selection::range(5, 10, 3, 2, SelectionMode::Char);
        assert!(sel.contains(3, 2));
        assert!(sel.contains(4, 0));
        assert!(sel.contains(5, 9));
        assert!(!sel.contains(5, 10));
    }

    #[test]
    fn test_selection_line_mode() {
        let sel = Selection::range(2, 0, 4, 0, SelectionMode::Line);
        assert!(sel.contains_line(2));
        assert!(sel.contains_line(3));
        assert!(sel.contains_line(4));
        assert!(!sel.contains_line(1));
        assert!(!sel.contains_line(5));
    }

    #[test]
    fn test_selection_block_mode() {
        // Block select columns 2-5 on lines 1-3
        let sel = Selection::range(1, 2, 3, 5, SelectionMode::Block);
        assert!(sel.contains(1, 3));  // in block
        assert!(sel.contains(2, 4));  // in block
        assert!(!sel.contains(1, 1)); // left of block
        assert!(!sel.contains(1, 5)); // right of block (end exclusive)
    }

    #[test]
    fn test_normalized_range() {
        let sel = Selection::range(5, 10, 3, 2, SelectionMode::Char);
        let (sl, sc, el, ec) = sel.normalized_range();
        assert_eq!((sl, sc, el, ec), (3, 2, 5, 10));
    }

    #[test]
    fn test_cursor_contains_nothing() {
        let sel = Selection::cursor(3, 5);
        assert!(!sel.contains(3, 5));
        assert!(!sel.contains_line(3)); // collapsed = nothing selected
    }
}
