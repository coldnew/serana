use crate::types::Color;

/// A single cell in the screen buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenCell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
    pub blink: bool,
    pub underline_color: Option<Color>,
    pub hyperlink: Option<u32>,
}

impl ScreenCell {
    pub const EMPTY: Self = Self {
        ch: ' ',
        fg: Color::WHITE,
        bg: Color::BLACK,
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        dim: false,
        reverse: false,
        blink: false,
        underline_color: None,
        hyperlink: None,
    };

    pub fn new(ch: char, fg: Color, bg: Color) -> Self {
        Self {
            ch,
            fg,
            bg,
            ..Self::EMPTY
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ch == ' ' && !self.bold && !self.italic && !self.underline && !self.strikethrough
    }
}

impl Default for ScreenCell {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Flat screen buffer with cell-level diffing.
///
/// Like storm-rs's ScreenBuffer, this stores cells in a contiguous Vec
/// and supports damage tracking for efficient partial updates.
#[derive(Debug, Clone)]
pub struct ScreenBuffer {
    pub width: u16,
    pub height: u16,
    cells: Vec<ScreenCell>,
    /// Whether any cell has been modified since last `clear_damage()`.
    has_damage: bool,
    /// Bounding box of damaged region.
    damage_x1: u16,
    damage_y1: u16,
    damage_x2: u16,
    damage_y2: u16,
}

impl ScreenBuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            cells: vec![ScreenCell::EMPTY; size],
            has_damage: false,
            damage_x1: width,
            damage_y1: height,
            damage_x2: 0,
            damage_y2: 0,
        }
    }

    #[inline]
    fn idx(&self, x: u16, y: u16) -> usize {
        (y as usize) * (self.width as usize) + (x as usize)
    }

    /// Get a cell at (x, y).
    pub fn get(&self, x: u16, y: u16) -> ScreenCell {
        if x >= self.width || y >= self.height {
            return ScreenCell::EMPTY;
        }
        self.cells[self.idx(x, y)]
    }

    /// Set a cell at (x, y) with damage tracking.
    pub fn set(&mut self, x: u16, y: u16, cell: ScreenCell) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.idx(x, y);
        if self.cells[idx] != cell {
            self.cells[idx] = cell;
            self.mark_damaged(x, y);
        }
    }

    /// Set a character at (x, y) with style attributes.
    pub fn set_char(
        &mut self,
        x: u16,
        y: u16,
        ch: char,
        fg: Color,
        bg: Color,
        bold: bool,
        dim: bool,
        underline: bool,
        strikethrough: bool,
        italic: bool,
        reverse: bool,
    ) {
        self.set(
            x,
            y,
            ScreenCell {
                ch,
                fg,
                bg,
                bold,
                italic,
                underline,
                strikethrough,
                dim,
                reverse,
                blink: false,
                underline_color: None,
                hyperlink: None,
            },
        );
    }

    /// Write a string starting at (x, y) with basic style (bold/dim only).
    pub fn write_str(
        &mut self,
        x: u16,
        y: u16,
        s: &str,
        fg: Color,
        bg: Color,
        bold: bool,
        dim: bool,
    ) {
        for (i, ch) in s.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= self.width {
                break;
            }
            self.set_char(cx, y, ch, fg, bg, bold, dim, false, false, false, false);
        }
    }

    /// Write a string with full style attributes.
    pub fn write_styled(&mut self, x: u16, y: u16, s: &str, style: &crate::types::Style) {
        let fg = style.fg.unwrap_or(Color::WHITE);
        let bg = style.bg.unwrap_or(Color::BLACK);
        for (i, ch) in s.chars().enumerate() {
            let cx = x + i as u16;
            if cx >= self.width {
                break;
            }
            self.set(
                cx,
                y,
                ScreenCell {
                    ch,
                    fg,
                    bg,
                    bold: style.bold,
                    italic: style.italic,
                    underline: style.underline,
                    strikethrough: style.strikethrough,
                    dim: style.dim,
                    reverse: style.reverse,
                    blink: style.blink,
                    underline_color: style.underline_color,
                    hyperlink: None,
                },
            );
        }
    }

    /// Fill a rectangular region with a cell.
    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, cell: ScreenCell) {
        for dy in 0..h {
            for dx in 0..w {
                self.set(x + dx, y + dy, cell);
            }
        }
    }

    /// Fill a rectangular region with a character.
    pub fn fill_char(&mut self, x: u16, y: u16, w: u16, h: u16, ch: char, fg: Color, bg: Color) {
        self.fill_rect(x, y, w, h, ScreenCell::new(ch, fg, bg));
    }

    /// Draw a horizontal line.
    pub fn hline(&mut self, x: u16, y: u16, width: u16, ch: char, fg: Color, bg: Color) {
        for dx in 0..width {
            self.set_char(
                x + dx,
                y,
                ch,
                fg,
                bg,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }
    }

    /// Draw a border around a region.
    pub fn draw_border(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        fg: Color,
        bg: Color,
        title: Option<&str>,
    ) {
        if w < 2 || h < 2 {
            return;
        }

        let tl = '┌';
        let tr = '┐';
        let bl = '└';
        let br = '┘';
        let h_line = '─';
        let v_line = '│';

        // Top
        self.set_char(x, y, tl, fg, bg, false, false, false, false, false, false);
        for dx in 1..w - 1 {
            self.set_char(
                x + dx,
                y,
                h_line,
                fg,
                bg,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }
        self.set_char(
            x + w - 1,
            y,
            tr,
            fg,
            bg,
            false,
            false,
            false,
            false,
            false,
            false,
        );

        // Title
        if let Some(title) = title {
            let title_x = x + 2;
            let max_len = (w - 3) as usize;
            let truncated: String = title.chars().take(max_len).collect();
            self.write_str(title_x, y, &truncated, fg, bg, true, false);
        }

        // Sides
        for dy in 1..h - 1 {
            self.set_char(
                x,
                y + dy,
                v_line,
                fg,
                bg,
                false,
                false,
                false,
                false,
                false,
                false,
            );
            self.set_char(
                x + w - 1,
                y + dy,
                v_line,
                fg,
                bg,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }

        // Bottom
        self.set_char(
            x,
            y + h - 1,
            bl,
            fg,
            bg,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        for dx in 1..w - 1 {
            self.set_char(
                x + dx,
                y + h - 1,
                h_line,
                fg,
                bg,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }
        self.set_char(
            x + w - 1,
            y + h - 1,
            br,
            fg,
            bg,
            false,
            false,
            false,
            false,
            false,
            false,
        );
    }

    /// Clear the entire buffer.
    pub fn clear(&mut self) {
        self.cells.fill(ScreenCell::EMPTY);
        self.has_damage = true;
        self.damage_x1 = 0;
        self.damage_y1 = 0;
        self.damage_x2 = self.width;
        self.damage_y2 = self.height;
    }

    /// Mark a cell as damaged.
    fn mark_damaged(&mut self, x: u16, y: u16) {
        self.has_damage = true;
        self.damage_x1 = self.damage_x1.min(x);
        self.damage_y1 = self.damage_y1.min(y);
        self.damage_x2 = self.damage_x2.max(x + 1);
        self.damage_y2 = self.damage_y2.max(y + 1);
    }

    /// Check if there's any damage.
    pub fn has_damage(&self) -> bool {
        self.has_damage
    }

    /// Get the damaged bounding box (x1, y1, x2, y2).
    pub fn damage_rect(&self) -> (u16, u16, u16, u16) {
        (
            self.damage_x1,
            self.damage_y1,
            self.damage_x2,
            self.damage_y2,
        )
    }

    /// Clear damage tracking. Call after rendering.
    pub fn clear_damage(&mut self) {
        self.has_damage = false;
        self.damage_x1 = self.width;
        self.damage_y1 = self.height;
        self.damage_x2 = 0;
        self.damage_y2 = 0;
    }
    /// Write a StyledLine starting at (x, y), with fallback fg/bg for unstyled spans.
    pub fn write_styled_line(
        &mut self,
        x: u16,
        y: u16,
        line: &crate::types::StyledLine,
        default_fg: Color,
        default_bg: Color,
    ) {
        let mut cx = x;
        for span in &line.spans {
            let fg = span.style.fg.unwrap_or(default_fg);
            let bg = span.style.bg.unwrap_or(default_bg);
            for ch in span.text.chars() {
                if cx >= self.width {
                    return;
                }
                self.set(
                    cx,
                    y,
                    ScreenCell {
                        ch,
                        fg,
                        bg,
                        bold: span.style.bold,
                        italic: span.style.italic,
                        underline: span.style.underline,
                        strikethrough: span.style.strikethrough,
                        dim: span.style.dim,
                        reverse: span.style.reverse,
                        blink: span.style.blink,
                        underline_color: span.style.underline_color,
                        hyperlink: None,
                    },
                );
                cx += 1;
            }
        }
    }

    /// Diff against a previous buffer, returning only changed cells.
    pub fn diff(&self, prev: &ScreenBuffer) -> Vec<(u16, u16, ScreenCell)> {
        let mut changes = Vec::new();
        let w = self.width.min(prev.width);
        let h = self.height.min(prev.height);
        for y in 0..h {
            for x in 0..w {
                let new = self.get(x, y);
                let old = prev.get(x, y);
                if new != old {
                    changes.push((x, y, new));
                }
            }
        }
        // Handle size differences
        if self.height > prev.height || self.width > prev.width {
            for y in 0..self.height {
                for x in 0..self.width {
                    if x >= prev.width || y >= prev.height {
                        changes.push((x, y, self.get(x, y)));
                    }
                }
            }
        }
        changes
    }

    /// Resize the buffer, preserving existing content where possible.
    pub fn resize(&mut self, new_width: u16, new_height: u16) {
        let old = self.clone();
        let size = (new_width as usize) * (new_height as usize);
        self.cells = vec![ScreenCell::EMPTY; size];
        self.width = new_width;
        self.height = new_height;

        let copy_w = old.width.min(new_width);
        let copy_h = old.height.min(new_height);
        for y in 0..copy_h {
            for x in 0..copy_w {
                let idx = self.idx(x, y);
                self.cells[idx] = old.get(x, y);
            }
        }
        self.has_damage = true;
        self.damage_x1 = 0;
        self.damage_y1 = 0;
        self.damage_x2 = new_width;
        self.damage_y2 = new_height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut buf = ScreenBuffer::new(10, 5);
        buf.set_char(
            3,
            2,
            'X',
            Color::RED,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert_eq!(buf.get(3, 2).ch, 'X');
        assert_eq!(buf.get(3, 2).fg, Color::RED);
    }

    #[test]
    fn test_damage_tracking() {
        let mut buf = ScreenBuffer::new(10, 5);
        assert!(!buf.has_damage());
        buf.set_char(
            5,
            3,
            'A',
            Color::WHITE,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert!(buf.has_damage());
        let (x1, y1, x2, y2) = buf.damage_rect();
        assert_eq!((x1, y1, x2, y2), (5, 3, 6, 4));
    }

    #[test]
    fn test_diff() {
        let mut buf1 = ScreenBuffer::new(5, 3);
        let mut buf2 = ScreenBuffer::new(5, 3);
        buf2.set_char(
            2,
            1,
            'X',
            Color::RED,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        let changes = buf2.diff(&buf1);
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0],
            (2, 1, ScreenCell::new('X', Color::RED, Color::BLACK))
        );
    }

    #[test]
    fn test_write_str() {
        let mut buf = ScreenBuffer::new(20, 5);
        buf.write_str(2, 1, "Hello", Color::GREEN, Color::BLACK, true, false);
        assert_eq!(buf.get(2, 1).ch, 'H');
        assert_eq!(buf.get(6, 1).ch, 'o');
        assert!(buf.get(2, 1).bold);
    }

    #[test]
    fn test_border() {
        let mut buf = ScreenBuffer::new(10, 5);
        buf.draw_border(0, 0, 10, 5, Color::WHITE, Color::BLACK, None);
        assert_eq!(buf.get(0, 0).ch, '┌');
        assert_eq!(buf.get(9, 0).ch, '┐');
        assert_eq!(buf.get(0, 4).ch, '└');
        assert_eq!(buf.get(9, 4).ch, '┘');
    }

    #[test]
    fn test_resize() {
        let mut buf = ScreenBuffer::new(5, 3);
        buf.set_char(
            2,
            1,
            'X',
            Color::RED,
            Color::BLACK,
            false,
            false,
            false,
            false,
            false,
            false,
        );
        buf.resize(10, 5);
        assert_eq!(buf.get(2, 1).ch, 'X');
        assert_eq!(buf.width, 10);
        assert_eq!(buf.height, 5);
    }

    #[test]
    fn test_write_styled() {
        use crate::types::Style;
        let mut buf = ScreenBuffer::new(20, 5);
        let style = Style::default()
            .bold()
            .italic()
            .underline()
            .fg(Color::new(255, 128, 0));
        buf.write_styled(1, 1, "Test", &style);
        let cell = buf.get(1, 1);
        assert_eq!(cell.ch, 'T');
        assert!(cell.bold);
        assert!(cell.italic);
        assert!(cell.underline);
        assert_eq!(cell.fg, Color::new(255, 128, 0));
    }
}
