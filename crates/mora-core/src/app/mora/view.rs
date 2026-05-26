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
        if buf.cursor.row < self.scroll_top {
            self.scroll_top = buf.cursor.row;
        } else if buf.cursor.row >= self.scroll_top + self.height {
            self.scroll_top = buf.cursor.row - self.height + 1;
        }
        let digits = buf.line_count().to_string().len().max(3);
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
