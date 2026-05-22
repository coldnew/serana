//! Multi-line input editor with undo support.

use std::collections::VecDeque;

const MAX_UNDO_HISTORY: usize = 100;

/// A multi-line text editor with cursor tracking and undo.
#[derive(Debug)]
pub struct Editor {
    lines: Vec<String>,
    /// Row index (line number).
    row: usize,
    /// Column index (byte offset within current line).
    col: usize,
    /// Undo stack: snapshots of (lines, row, col).
    undo_stack: VecDeque<(Vec<String>, usize, usize)>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            row: 0,
            col: 0,
            undo_stack: VecDeque::new(),
        }
    }

    /// All lines as a single string (joined by newline).
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Set content from a single string.
    pub fn set_content(&mut self, text: &str) {
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|l| l.to_string()).collect()
        };
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.row = self.lines.len() - 1;
        self.col = self.lines[self.row].len();
        self.undo_stack.clear();
    }

    /// Clear all content.
    pub fn clear(&mut self) {
        self.save_undo();
        self.lines = vec![String::new()];
        self.row = 0;
        self.col = 0;
    }

    /// Is the editor empty?
    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    /// Number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Current row.
    pub fn row(&self) -> usize {
        self.row
    }

    /// Current column.
    pub fn col(&self) -> usize {
        self.col
    }

    /// Get a specific line.
    pub fn line(&self, idx: usize) -> &str {
        &self.lines[idx]
    }

    /// Current line.
    pub fn current_line(&self) -> &str {
        &self.lines[self.row]
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, ch: char) {
        self.save_undo();
        self.lines[self.row].insert(self.col, ch);
        self.col += ch.len_utf8();
    }

    /// Insert a newline (Shift+Enter).
    pub fn insert_newline(&mut self) {
        self.save_undo();
        let rest = self.lines[self.row][self.col..].to_string();
        self.lines[self.row].truncate(self.col);
        self.row += 1;
        self.lines.insert(self.row, rest);
        self.col = 0;
    }

    /// Delete character before cursor (Backspace).
    pub fn delete_backward(&mut self) {
        if self.col > 0 {
            self.save_undo();
            let prev_char_len = self.lines[self.row][..self.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.col -= prev_char_len;
            self.lines[self.row].remove(self.col);
        } else if self.row > 0 {
            // Merge with previous line
            self.save_undo();
            let current = self.lines.remove(self.row);
            self.row -= 1;
            self.col = self.lines[self.row].len();
            self.lines[self.row].push_str(&current);
        }
    }

    /// Delete character at cursor (Delete).
    pub fn delete_forward(&mut self) {
        if self.col < self.lines[self.row].len() {
            self.save_undo();
            self.lines[self.row].remove(self.col);
        } else if self.row + 1 < self.lines.len() {
            // Merge next line into current
            self.save_undo();
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.col > 0 {
            let prev_char_len = self.lines[self.row][..self.col]
                .chars()
                .next_back()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.col -= prev_char_len;
        } else if self.row > 0 {
            self.row -= 1;
            self.col = self.lines[self.row].len();
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        if self.col < self.lines[self.row].len() {
            let next_char_len = self.lines[self.row][self.col..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.col += next_char_len;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = 0;
        }
    }

    /// Move cursor up.
    pub fn move_up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.col = self.col.min(self.lines[self.row].len());
        }
    }

    /// Move cursor down.
    pub fn move_down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.col = self.col.min(self.lines[self.row].len());
        }
    }

    /// Move cursor to start of line.
    pub fn move_home(&mut self) {
        self.col = 0;
    }

    /// Move cursor to end of line.
    pub fn move_end(&mut self) {
        self.col = self.lines[self.row].len();
    }

    /// Undo last change.
    pub fn undo(&mut self) {
        if let Some((lines, row, col)) = self.undo_stack.pop_back() {
            self.lines = lines;
            self.row = row;
            self.col = col;
        }
    }

    fn save_undo(&mut self) {
        self.undo_stack
            .push_back((self.lines.clone(), self.row, self.col));
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert() {
        let mut e = Editor::new();
        e.insert_char('h');
        e.insert_char('i');
        assert_eq!(e.content(), "hi");
        assert_eq!(e.col, 2);
    }

    #[test]
    fn test_newline() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_newline();
        e.insert_char('b');
        assert_eq!(e.content(), "a\nb");
        assert_eq!(e.line_count(), 2);
    }

    #[test]
    fn test_backspace_merge_lines() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_newline();
        e.insert_char('b');
        assert_eq!(e.content(), "a\nb");
        e.delete_backward(); // merge 'b' into empty space, removes newline
        assert_eq!(e.content(), "ab");
    }

    #[test]
    fn test_cursor_navigation() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_newline();
        e.insert_char('b');
        assert_eq!(e.row, 1);
        e.move_up();
        assert_eq!(e.row, 0);
        e.move_down();
        assert_eq!(e.row, 1);
    }

    #[test]
    fn test_undo() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('b');
        assert_eq!(e.content(), "ab");
        e.undo();
        assert_eq!(e.content(), "a");
        e.undo();
        assert_eq!(e.content(), "");
    }

    #[test]
    fn test_insert_middle() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('c');
        // cursor is at end (after 'c'), move left
        e.move_left();
        e.insert_char('b');
        assert_eq!(e.content(), "abc");
    }

    #[test]
    fn test_delete_forward() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('b');
        e.insert_char('c');
        e.move_left();
        e.move_left(); // cursor after 'a'
        e.delete_forward();
        assert_eq!(e.content(), "ac");
    }

    #[test]
    fn test_home_end() {
        let mut e = Editor::new();
        e.insert_char('a');
        e.insert_char('b');
        e.move_home();
        assert_eq!(e.col, 0);
        e.move_end();
        assert_eq!(e.col, 2);
    }

    #[test]
    fn test_empty_is_empty() {
        let e = Editor::new();
        assert!(e.is_empty());
    }

    #[test]
    fn test_set_content() {
        let mut e = Editor::new();
        e.set_content("hello\nworld");
        assert_eq!(e.content(), "hello\nworld");
        assert_eq!(e.line_count(), 2);
    }
}
