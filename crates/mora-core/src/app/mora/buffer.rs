use std::collections::VecDeque;
use std::path::PathBuf;

const MAX_UNDO: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<String>,
    cursor: Cursor,
}

#[derive(Debug)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub scroll_offset: usize,
    pub modified: bool,
    pub path: Option<PathBuf>,
    undo_stack: VecDeque<Snapshot>,
    redo_stack: Vec<Snapshot>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0,
            modified: false,
            path: None,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        Ok(Self {
            lines,
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0,
            modified: false,
            path: Some(path.to_path_buf()),
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.path {
            std::fs::write(path, self.content())?;
            self.modified = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.content())?;
        self.path = Some(path.to_path_buf());
        self.modified = false;
        Ok(())
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn current_line(&self) -> &str {
        &self.lines[self.cursor.row]
    }

    pub fn line(&self, idx: usize) -> &str {
        &self.lines[idx]
    }

    pub fn filename(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            cursor: self.cursor.clone(),
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push_back(self.snapshot());
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(snap) = self.undo_stack.pop_back() {
            self.redo_stack.push(self.snapshot());
            self.lines = snap.lines;
            self.cursor = snap.cursor;
            self.clamp_cursor();
        }
    }

    pub fn redo(&mut self) {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push_back(self.snapshot());
            self.lines = snap.lines;
            self.cursor = snap.cursor;
            self.clamp_cursor();
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor.row = self.cursor.row.min(self.lines.len().saturating_sub(1));
        self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
    }

    fn col_byte_index(line: &str, char_idx: usize) -> usize {
        line.char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(line.len())
    }

    fn char_index_at_col(line: &str, col: usize) -> usize {
        line[..col].chars().count()
    }

    pub fn insert_char(&mut self, ch: char) {
        self.push_undo();
        let line = &mut self.lines[self.cursor.row];
        line.insert(self.cursor.col, ch);
        self.cursor.col += ch.len_utf8();
        self.modified = true;
    }

    pub fn insert_string(&mut self, s: &str) {
        self.push_undo();
        for ch in s.chars() {
            if ch == '\n' {
                self.insert_newline_no_undo();
            } else {
                let line = &mut self.lines[self.cursor.row];
                line.insert(self.cursor.col, ch);
                self.cursor.col += ch.len_utf8();
            }
        }
        self.modified = true;
    }

    pub fn insert_newline(&mut self) {
        self.push_undo();
        self.insert_newline_no_undo();
        self.modified = true;
    }

    fn insert_newline_no_undo(&mut self) {
        let rest = self.lines[self.cursor.row][self.cursor.col..].to_string();
        self.lines[self.cursor.row].truncate(self.cursor.col);
        self.cursor.row += 1;
        self.lines.insert(self.cursor.row, rest);
        self.cursor.col = 0;
    }

    pub fn delete_backward(&mut self) {
        if self.cursor.col > 0 {
            self.push_undo();
            let line = &mut self.lines[self.cursor.row];
            let prev_len = line[..self.cursor.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col -= prev_len;
            line.remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.row > 0 {
            self.push_undo();
            let current = self.lines.remove(self.cursor.row);
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
            self.lines[self.cursor.row].push_str(&current);
            self.modified = true;
        }
    }

    pub fn delete_forward(&mut self) {
        let line_len = self.lines[self.cursor.row].len();
        if self.cursor.col < line_len {
            self.push_undo();
            self.lines[self.cursor.row].remove(self.cursor.col);
            self.modified = true;
        } else if self.cursor.row + 1 < self.lines.len() {
            self.push_undo();
            let next = self.lines.remove(self.cursor.row + 1);
            self.lines[self.cursor.row].push_str(&next);
            self.modified = true;
        }
    }

    pub fn delete_line(&mut self) {
        self.push_undo();
        if self.lines.len() == 1 {
            self.lines[0].clear();
            self.cursor.col = 0;
        } else {
            self.lines.remove(self.cursor.row);
            if self.cursor.row >= self.lines.len() {
                self.cursor.row = self.lines.len() - 1;
            }
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
        }
        self.modified = true;
    }

    pub fn delete_to_eol(&mut self) {
        self.push_undo();
        let line = &mut self.lines[self.cursor.row];
        line.truncate(self.cursor.col);
        self.modified = true;
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            let line = &self.lines[self.cursor.row];
            let prev_len = line[..self.cursor.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col -= prev_len;
        } else if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].len();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor.row].len();
        if self.cursor.col < line_len {
            let line = &self.lines[self.cursor.row];
            let next_len = line[self.cursor.col..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor.col += next_len;
        } else if self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = self.cursor.col.min(self.lines[self.cursor.row].len());
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.row].len();
    }

    pub fn move_to_file_start(&mut self) {
        self.cursor.row = 0;
        self.cursor.col = 0;
    }

    pub fn move_to_file_end(&mut self) {
        self.cursor.row = self.lines.len() - 1;
        self.cursor.col = self.lines[self.cursor.row].len();
    }

    pub fn move_to_line(&mut self, line_num: usize) {
        self.cursor.row = line_num.min(self.lines.len().saturating_sub(1));
        self.cursor.col = 0;
    }

    pub fn move_word_forward(&mut self) {
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, self.cursor.col);
        let mut i = char_idx;
        let len = chars.len();

        while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
        while i < len && !chars[i].is_alphanumeric() && chars[i] != '_' {
            i += 1;
        }

        if i >= len && self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        } else {
            self.cursor.col = Self::col_byte_index(line, i);
        }
    }

    pub fn move_word_end(&mut self) {
        let line = self.lines[self.cursor.row].clone();
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(&line, self.cursor.col);
        let len = chars.len();
        if len == 0 || char_idx >= len {
            if self.cursor.row + 1 < self.lines.len() {
                self.cursor.row += 1;
                self.cursor.col = 0;
                self.move_word_end();
            }
            return;
        }

        let mut i = char_idx;
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            if self.cursor.row + 1 < self.lines.len() {
                self.cursor.row += 1;
                self.cursor.col = 0;
                self.move_word_end();
            }
            return;
        }

        let is_word = chars[i].is_alphanumeric() || chars[i] == '_';
        while i < len {
            let c = chars[i];
            if c.is_whitespace() || is_word != (c.is_alphanumeric() || c == '_') {
                break;
            }
            i += 1;
        }

        let last = i.saturating_sub(1);
        self.cursor.col = Self::col_byte_index(&line, last);
    }

    pub fn move_word_backward(&mut self) {
        let line = &self.lines[self.cursor.row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, self.cursor.col);
        let mut i = char_idx;

        if i == 0 {
            if self.cursor.row > 0 {
                self.cursor.row -= 1;
                self.cursor.col = self.lines[self.cursor.row].len();
            }
            return;
        }

        i -= 1;
        while i > 0 && !chars[i].is_alphanumeric() && chars[i] != '_' {
            i -= 1;
        }
        while i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
            i -= 1;
        }

        self.cursor.col = Self::col_byte_index(line, i);
    }

    pub fn goto_line(&mut self, line_num: usize) {
        self.cursor.row = (line_num - 1).min(self.lines.len().saturating_sub(1));
        self.cursor.col = 0;
    }

    pub fn find_forward(&mut self, query: &str) -> bool {
        let start_row = self.cursor.row;
        let start_col = self.cursor.col + 1;

        for row_off in 0..self.lines.len() {
            let row = (start_row + row_off) % self.lines.len();
            let search_col = if row_off == 0 { start_col } else { 0 };
            if let Some(idx) = self.lines[row][search_col..].find(query) {
                self.cursor.row = row;
                self.cursor.col = search_col + idx;
                return true;
            }
        }
        false
    }

    pub fn find_backward(&mut self, query: &str) -> bool {
        let start_row = self.cursor.row;
        for row_off in 0..self.lines.len() {
            let row = (start_row + self.lines.len() - row_off) % self.lines.len();
            let search_end = if row_off == 0 {
                self.cursor.col
            } else {
                self.lines[row].len()
            };
            if let Some(idx) = self.lines[row][..search_end].rfind(query) {
                self.cursor.row = row;
                self.cursor.col = idx;
                return true;
            }
        }
        false
    }

    pub fn replace_first(&mut self, old: &str, new: &str) -> bool {
        let line = &self.lines[self.cursor.row];
        if let Some(idx) = line[self.cursor.col..].find(old) {
            self.push_undo();
            let abs_idx = self.cursor.col + idx;
            self.lines[self.cursor.row].replace_range(abs_idx..abs_idx + old.len(), new);
            self.modified = true;
            return true;
        }
        for row in (self.cursor.row + 1)..self.lines.len() {
            if let Some(idx) = self.lines[row].find(old) {
                self.push_undo();
                self.lines[row].replace_range(idx..idx + old.len(), new);
                self.cursor.row = row;
                self.cursor.col = idx;
                self.modified = true;
                return true;
            }
        }
        false
    }

    pub fn push_undo_snapshot(&mut self) {
        self.push_undo();
    }

    pub fn replace_all(&mut self, old: &str, new: &str) -> usize {
        self.push_undo();
        let mut count = 0;
        for line in &mut self.lines {
            while let Some(idx) = line.find(old) {
                line.replace_range(idx..idx + old.len(), new);
                count += 1;
            }
        }
        if count > 0 {
            self.modified = true;
            self.clamp_cursor();
        }
        count
    }

    pub fn transpose_char(&mut self) {
        let row = self.cursor.row;
        let col = self.cursor.col;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let char_idx = Self::char_index_at_col(line, col);

        if chars.len() < 2 || char_idx == 0 {
            return;
        }

        let swap_idx = if char_idx >= chars.len() {
            chars.len() - 2
        } else {
            char_idx - 1
        };

        if swap_idx + 1 >= chars.len() {
            return;
        }

        self.push_undo();
        let mut new_chars = chars.clone();
        new_chars.swap(swap_idx, swap_idx + 1);
        self.lines[row] = new_chars.into_iter().collect();
        self.cursor.col = Self::col_byte_index(&self.lines[row], swap_idx + 2);
        self.modified = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_cursor() {
        let mut buf = Buffer::new();
        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.content(), "hi");
        assert_eq!(buf.cursor.col, 2);
    }

    #[test]
    fn test_newline() {
        let mut buf = Buffer::new();
        buf.insert_char('a');
        buf.insert_newline();
        buf.insert_char('b');
        assert_eq!(buf.content(), "a\nb");
        assert_eq!(buf.line_count(), 2);
    }

    #[test]
    fn test_delete_backward_merge() {
        let mut buf = Buffer::new();
        buf.insert_string("ab\ncd");
        buf.cursor.row = 1;
        buf.cursor.col = 0;
        buf.delete_backward();
        assert_eq!(buf.content(), "abcd");
    }

    #[test]
    fn test_word_movement() {
        let mut buf = Buffer::new();
        buf.insert_string("hello world foo");
        buf.cursor.col = 0;
        buf.move_word_forward();
        assert_eq!(buf.cursor.col, 6);
        buf.move_word_forward();
        assert_eq!(buf.cursor.col, 12);
    }

    #[test]
    fn test_find_forward() {
        let mut buf = Buffer::new();
        buf.insert_string("hello\nworld\nhello again");
        buf.cursor = Cursor { row: 0, col: 0 };
        assert!(buf.find_forward("world"));
        assert_eq!(buf.cursor.row, 1);
        assert!(buf.find_forward("hello"));
        assert_eq!(buf.cursor.row, 2);
    }

    #[test]
    fn test_replace_all() {
        let mut buf = Buffer::new();
        buf.insert_string("foo bar foo baz foo");
        let count = buf.replace_all("foo", "qux");
        assert_eq!(count, 3);
        assert_eq!(buf.content(), "qux bar qux baz qux");
    }

    #[test]
    fn test_undo_redo() {
        let mut buf = Buffer::new();
        buf.insert_string("hello");
        buf.undo();
        assert_eq!(buf.content(), "");
        buf.redo();
        assert_eq!(buf.content(), "hello");
    }

    #[test]
    fn test_delete_line() {
        let mut buf = Buffer::new();
        buf.insert_string("line1\nline2\nline3");
        buf.cursor.row = 1;
        buf.delete_line();
        assert_eq!(buf.content(), "line1\nline3");
    }

    #[test]
    fn test_goto_line() {
        let mut buf = Buffer::new();
        buf.insert_string("a\nb\nc\nd\ne");
        buf.goto_line(3);
        assert_eq!(buf.cursor.row, 2);
    }
}
