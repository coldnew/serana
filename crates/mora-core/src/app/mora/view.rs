use crate::app::mora::buffer::Buffer;

#[derive(Debug)]
pub struct View {
    pub scroll_top: usize,
    pub height: usize,
    pub gutter_width: u16,
}

impl View {
    pub fn new(height: usize) -> Self {
        Self {
            scroll_top: 0,
            height: height.max(1),
            gutter_width: 4,
        }
    }

    pub fn ensure_cursor_visible(&mut self, buf: &Buffer) {
        let (min_row, max_row) = if let (Some(start), Some(end)) = (buf.narrow_start, buf.narrow_end) {
            (start, end)
        } else {
            (0, buf.line_count().saturating_sub(1))
        };
        if buf.cursor.row < self.scroll_top || buf.cursor.row < min_row {
            self.scroll_top = buf.cursor.row.max(min_row);
        } else if buf.cursor.row >= self.scroll_top + self.height || buf.cursor.row > max_row {
            self.scroll_top = buf.cursor.row.saturating_sub(self.height + 1).min(max_row);
        }
        let max_line = max_row + 1;
        let digits = max_line.to_string().len().max(3);
        self.gutter_width = digits as u16 + 1;
    }

    pub fn scroll(&mut self, delta: isize, total_lines: usize) {
        if delta > 0 {
            self.scroll_top = (self.scroll_top + delta as usize).min(total_lines.saturating_sub(1));
        } else {
            self.scroll_top = self.scroll_top.saturating_sub((-delta) as usize);
        }
    }

    pub fn visible_range(&self, total_lines: usize) -> (usize, usize) {
        let start = self.scroll_top;
        let end = (start + self.height).min(total_lines);
        (start, end)
    }

    pub fn cursor_view_row(&self, buf: &Buffer) -> usize {
        buf.cursor.row.saturating_sub(self.scroll_top)
    }
}
