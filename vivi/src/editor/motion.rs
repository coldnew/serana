use crate::editor::Editor;

impl Editor {
    pub(super) fn move_word_forward(&mut self) {
        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();
        let len = bytes.len();

        let mut col = self.cursor.col;
        let mut in_word = col < len && is_word_byte(bytes[col]);

        while col < len {
            if in_word && !is_word_byte(bytes[col]) {
                in_word = false;
            } else if !in_word && is_word_byte(bytes[col]) {
                break;
            }
            col += 1;
        }

        if col < len {
            self.cursor.col = col;
        } else if self.cursor.row + 1 < self.buffer.line_count() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
    }

    pub(super) fn move_word_backward(&mut self) {
        if self.cursor.col == 0 {
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                let len = self.buffer.line_len(self.cursor.row);
                self.cursor.col = if len > 0 { len - 1 } else { 0 };
            }
            return;
        }

        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();

        let mut col = self.cursor.col.saturating_sub(1);

        while col > 0 && !is_word_byte(bytes[col]) {
            col -= 1;
        }

        while col > 0 && is_word_byte(bytes[col - 1]) {
            col -= 1;
        }

        self.cursor.col = col;
    }

    pub(super) fn move_word_end(&mut self) {
        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();
        let len = bytes.len();

        let mut col = self.cursor.col + 1;
        let mut in_word = false;

        while col < len {
            if !in_word && is_word_byte(bytes[col]) {
                in_word = true;
            } else if in_word && !is_word_byte(bytes[col]) {
                col -= 1;
                break;
            }
            col += 1;
        }

        if col >= len && len > 0 {
            col = len - 1;
        }

        if col < len {
            self.cursor.col = col;
        } else if self.cursor.row + 1 < self.buffer.line_count() {
            self.cursor.row += 1;
            self.cursor.col = 0;
            // Recurse to find end of first word on next line
            self.move_word_end();
        }
    }
}

#[inline]
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
