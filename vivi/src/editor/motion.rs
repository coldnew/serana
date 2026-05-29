use crate::editor::Editor;

impl Editor {
    pub(super) fn move_word_forward(&mut self) {
        let line = self.buffer.line(self.cursor.row);
        let chars: Vec<char> = line.chars().collect();

        let mut col = self.cursor.col;
        let mut in_word = col < chars.len() && is_word_char(chars[col]);

        while col < chars.len() {
            if in_word && !is_word_char(chars[col]) {
                in_word = false;
            } else if !in_word && is_word_char(chars[col]) {
                break;
            }
            col += 1;
        }

        if col < chars.len() {
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
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col.saturating_sub(1);

        while col > 0 && !is_word_char(chars[col]) {
            col -= 1;
        }

        while col > 0 && is_word_char(chars[col - 1]) {
            col -= 1;
        }

        self.cursor.col = col;
    }

    pub(super) fn move_word_end(&mut self) {
        let line = self.buffer.line(self.cursor.row);
        let chars: Vec<char> = line.chars().collect();

        let mut col = self.cursor.col + 1;
        let mut in_word = false;

        while col < chars.len() {
            if !in_word && is_word_char(chars[col]) {
                in_word = true;
            } else if in_word && !is_word_char(chars[col]) {
                col -= 1;
                break;
            }
            col += 1;
        }

        if col >= chars.len() && !chars.is_empty() {
            col = chars.len() - 1;
        }

        if col < chars.len() {
            self.cursor.col = col;
        } else if self.cursor.row + 1 < self.buffer.line_count() {
            self.cursor.row += 1;
            self.cursor.col = 0;
            self.move_word_end();
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
