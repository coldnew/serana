use display_protocol::{KeyCode, KeyEvent, KeyModifiers, Selection, SelectionMode};

use crate::buffer::Buffer;
use crate::cursor::Cursor;
use crate::mode::Mode;

mod commands;
mod motion;
mod visual;

/// The main editor state.
pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    /// Scroll offset (row, col) — the top-left corner of the visible area.
    scroll: ScrollOffset,
    /// Command line input text (when in Command mode).
    command_input: String,
    /// Cursor position within the command input.
    command_cursor: usize,
    /// Status message to display.
    message: Option<String>,
    /// Whether to quit.
    should_quit: bool,
    /// Terminal dimensions.
    term_width: u16,
    term_height: u16,
    /// Repeat count prefix (e.g., typing "5" before "j").
    count: Option<usize>,
    /// Yank register: stores a single yanked/deleted line or text.
    yank_register: YankRegister,
    /// Visual mode selection (anchor/head model).
    selection: Option<Selection>,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollOffset {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
enum YankRegister {
    Lines(Vec<String>),
    Chars(String),
}

impl YankRegister {
    fn is_lines(&self) -> bool {
        matches!(self, YankRegister::Lines(_))
    }
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::home(),
            mode: Mode::Normal,
            scroll: ScrollOffset { row: 0, col: 0 },
            command_input: String::new(),
            command_cursor: 0,
            message: None,
            should_quit: false,
            term_width: 80,
            term_height: 24,
            count: None,
            yank_register: YankRegister::Lines(vec![]),
            selection: None,
        }
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn scroll(&self) -> ScrollOffset {
        self.scroll
    }

    pub fn command_input(&self) -> &str {
        &self.command_input
    }

    pub fn command_cursor(&self) -> usize {
        self.command_cursor
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn set_term_size(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
    }

    /// Process a key event.
    pub fn handle_key(&mut self, key: KeyEvent) {
        self.message = None;

        match self.mode {
            Mode::Normal => self.handle_normal(key),
            Mode::Insert => self.handle_insert(key),
            Mode::Command => self.handle_command(key),
            Mode::Visual => self.handle_visual(key),
            Mode::VisualLine => self.handle_visual_line(key),
            Mode::Replace => self.handle_replace(key),
        }

        // Ensure cursor is in bounds after any operation
        self.clamp_cursor();
        self.update_scroll();
    }

    /// Get the effective count (defaults to 1).
    fn take_count(&mut self) -> usize {
        self.count.take().unwrap_or(1)
    }

    fn clamp_cursor(&mut self) {
        let line_count = self.buffer.line_count();
        if line_count == 0 {
            self.cursor = Cursor::home();
            return;
        }
        self.cursor
            .clamp(line_count, self.buffer.line_len(self.cursor.row));
    }

    fn update_scroll(&mut self) {
        let text_height = (self.term_height as usize).saturating_sub(2);
        let gutter =
            crate::render::line_number_width(self.buffer.line_count(), self.term_width as usize);
        let text_width = (self.term_width as usize).saturating_sub(gutter);

        // Vertical scroll
        if self.cursor.row < self.scroll.row {
            self.scroll.row = self.cursor.row;
        } else if self.cursor.row >= self.scroll.row + text_height {
            self.scroll.row = self.cursor.row - text_height + 1;
        }

        // Horizontal scroll
        if self.cursor.col < self.scroll.col {
            self.scroll.col = self.cursor.col;
        } else if self.cursor.col >= self.scroll.col + text_width {
            self.scroll.col = self.cursor.col - text_width + 1;
        }
    }

    // ─── Normal Mode ───────────────────────────────────────────────

    fn handle_normal(&mut self, key: KeyEvent) {
        match key.code {
            // Movement
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col > 0 {
                        self.cursor.col -= 1;
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.row + 1 < self.buffer.line_count() {
                        self.cursor.row += 1;
                        self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.row > 0 {
                        self.cursor.row -= 1;
                        self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
                    }
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.take_count();
                for _ in 0..n {
                    let max_col = self.buffer.line_len(self.cursor.row).saturating_sub(1);
                    if self.cursor.col < max_col {
                        self.cursor.col += 1;
                    }
                }
            }

            // Word motion
            KeyCode::Char('w') => {
                let n = self.take_count();
                for _ in 0..n {
                    self.move_word_forward();
                }
            }
            KeyCode::Char('b') => {
                let n = self.take_count();
                for _ in 0..n {
                    self.move_word_backward();
                }
            }
            KeyCode::Char('e') => {
                let n = self.take_count();
                for _ in 0..n {
                    self.move_word_end();
                }
            }

            // Line motion
            KeyCode::Char('0') => {
                self.cursor.col = 0;
            }
            KeyCode::Char('$') => {
                let len = self.buffer.line_len(self.cursor.row);
                self.cursor.col = if len > 0 { len - 1 } else { 0 };
            }
            KeyCode::Char('^') => {
                // First non-blank character
                let line = self.buffer.line(self.cursor.row);
                self.cursor.col = line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0);
            }
            KeyCode::Char('G') => {
                if self.count.is_some() {
                    // Go to specific line
                    let line = self.take_count();
                    self.cursor.row = (line - 1).min(self.buffer.line_count().saturating_sub(1));
                } else {
                    // Go to end of file
                    self.cursor.row = self.buffer.line_count().saturating_sub(1);
                }
                self.cursor.col = 0;
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::Char('g') => {
                // Check for 'gg'
                // We need a two-key sequence. For simplicity, treat the next
                // key immediately. In a real editor we'd have a pending state.
                // For now, gg goes to beginning of file.
                // We'll handle this as a special case - just 'g' once goes to top.
                // Actually let's make it work: g is pending, next g completes it.
                // To keep it simple, single 'g' goes to top.
                self.cursor = Cursor::home();
            }

            // Screen-relative motion
            KeyCode::Char('H') => {
                // Top of screen
                self.cursor.row = self.scroll.row;
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::Char('L') => {
                // Bottom of screen
                let text_height = (self.term_height as usize).saturating_sub(2);
                let bottom = (self.scroll.row + text_height - 1)
                    .min(self.buffer.line_count().saturating_sub(1));
                self.cursor.row = bottom;
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::Char('M') => {
                // Middle of screen
                let text_height = (self.term_height as usize).saturating_sub(2);
                let mid = self.scroll.row + text_height / 2;
                self.cursor.row = mid.min(self.buffer.line_count().saturating_sub(1));
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }

            // Page up/down
            KeyCode::PageDown | KeyCode::Char('\x06') => {
                // Ctrl-F
                let text_height = (self.term_height as usize).saturating_sub(2);
                let n = self.take_count();
                for _ in 0..n {
                    self.cursor.row = (self.cursor.row + text_height)
                        .min(self.buffer.line_count().saturating_sub(1));
                }
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::PageUp | KeyCode::Char('\x02') => {
                // Ctrl-B
                let text_height = (self.term_height as usize).saturating_sub(2);
                let n = self.take_count();
                for _ in 0..n {
                    self.cursor.row = self.cursor.row.saturating_sub(text_height);
                }
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }

            // Enter insert mode
            KeyCode::Char('i') => {
                self.mode = Mode::Insert;
            }
            KeyCode::Char('I') => {
                // Insert at beginning of line
                let line = self.buffer.line(self.cursor.row);
                self.cursor.col = line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0);
                self.mode = Mode::Insert;
            }
            KeyCode::Char('a') => {
                // Append after cursor
                let max_col = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < max_col {
                    self.cursor.col += 1;
                }
                self.mode = Mode::Insert;
            }
            KeyCode::Char('A') => {
                // Append at end of line
                self.cursor.col = self.buffer.line_len(self.cursor.row);
                self.mode = Mode::Insert;
            }
            KeyCode::Char('o') => {
                // Open line below
                let new_row = self.cursor.row + 1;
                self.buffer.insert_line(new_row, String::new());
                self.cursor.row = new_row;
                self.cursor.col = 0;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('O') => {
                // Open line above
                self.buffer.insert_line(self.cursor.row, String::new());
                self.cursor.col = 0;
                self.mode = Mode::Insert;
            }

            // Delete operations
            KeyCode::Char('x') => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col < self.buffer.line_len(self.cursor.row) {
                        self.buffer.delete_char(self.cursor.row, self.cursor.col);
                    }
                }
            }
            KeyCode::Char('X') => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col > 0 {
                        self.cursor.col -= 1;
                        self.buffer.delete_char(self.cursor.row, self.cursor.col);
                    }
                }
            }
            KeyCode::Char('d') => {
                // dd - delete line
                let n = self.take_count();
                let mut lines = Vec::new();
                for _ in 0..n {
                    if let Some(line) = self.buffer.delete_line(self.cursor.row) {
                        lines.push(line);
                    }
                }
                self.yank_register = YankRegister::Lines(lines);
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::Char('D') => {
                // Delete to end of line
                let len = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < len {
                    self.buffer
                        .replace_range(self.cursor.row, self.cursor.col, len, "");
                }
            }
            KeyCode::Char('C') => {
                // Change to end of line
                let len = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < len {
                    self.buffer
                        .replace_range(self.cursor.row, self.cursor.col, len, "");
                }
                self.mode = Mode::Insert;
            }
            KeyCode::Char('S') => {
                // Substitute entire line
                let _ = self.buffer.delete_line(self.cursor.row);
                self.buffer.insert_line(self.cursor.row, String::new());
                self.cursor.col = 0;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('s') => {
                // Substitute character
                if self.cursor.col < self.buffer.line_len(self.cursor.row) {
                    self.buffer.delete_char(self.cursor.row, self.cursor.col);
                }
                self.mode = Mode::Insert;
            }

            // Yank and paste
            KeyCode::Char('y') => {
                let n = self.take_count();
                let mut lines = Vec::new();
                for i in 0..n {
                    let row = self.cursor.row + i;
                    if row < self.buffer.line_count() {
                        lines.push(self.buffer.line(row).to_string());
                    }
                }
                self.yank_register = YankRegister::Lines(lines);
                self.message = Some(format!("{} line(s) yanked", n));
            }
            KeyCode::Char('Y') => {
                let line = self.buffer.line(self.cursor.row).to_string();
                self.yank_register = YankRegister::Chars(line);
                self.message = Some("1 line yanked".into());
            }
            KeyCode::Char('p') => {
                let n = self.take_count();
                match self.yank_register.clone() {
                    YankRegister::Lines(lines) => {
                        for _ in 0..n {
                            for (i, line) in lines.iter().enumerate() {
                                self.buffer
                                    .insert_line(self.cursor.row + 1 + i, line.clone());
                            }
                        }
                        self.cursor.row += 1;
                        self.cursor.col = 0;
                    }
                    YankRegister::Chars(text) => {
                        let col = self.cursor.col + 1;
                        for _ in 0..n {
                            self.buffer.insert_str(self.cursor.row, col, &text);
                        }
                    }
                }
            }
            KeyCode::Char('P') => {
                let n = self.take_count();
                match self.yank_register.clone() {
                    YankRegister::Lines(lines) => {
                        for _ in 0..n {
                            for (i, line) in lines.iter().enumerate() {
                                self.buffer.insert_line(self.cursor.row + i, line.clone());
                            }
                        }
                        self.cursor.col = 0;
                    }
                    YankRegister::Chars(text) => {
                        for _ in 0..n {
                            self.buffer
                                .insert_str(self.cursor.row, self.cursor.col, &text);
                        }
                    }
                }
            }

            // Undo — simple single-level
            // (We keep one undo level for now; real vim has unlimited)
            // Skip for MVP — too complex for now

            // Replace character
            KeyCode::Char('r') => {
                // We need the next character. For simplicity, enter replace-char
                // sub-mode. For MVP, we'll just not implement this two-key
                // sequence and show a message.
                self.message = Some("r requires next key (not yet implemented)".into());
            }

            // Enter replace mode
            KeyCode::Char('R') => {
                self.mode = Mode::Replace;
            }

            // Visual mode
            KeyCode::Char('v') => {
                self.mode = Mode::Visual;
                self.selection = Some(Selection::range(
                    self.cursor.row as u32,
                    self.cursor.col as u32,
                    self.cursor.row as u32,
                    self.cursor.col as u32,
                    SelectionMode::Char,
                ));
            }
            KeyCode::Char('V') => {
                self.mode = Mode::VisualLine;
                self.selection = Some(Selection::range(
                    self.cursor.row as u32,
                    0,
                    self.cursor.row as u32,
                    0,
                    SelectionMode::Line,
                ));
            }

            // Join lines
            KeyCode::Char('J') => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.buffer.join_lines(self.cursor.row).is_some() {
                        // Cursor stays at join point
                    }
                }
            }

            // Command mode
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command_input.clear();
                self.command_cursor = 0;
            }

            // Search
            KeyCode::Char('/') => {
                self.mode = Mode::Command;
                self.command_input = "/".into();
                self.command_cursor = 1;
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Command;
                self.command_input = "?".into();
                self.command_cursor = 1;
            }

            // Number prefix
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let digit = c.to_digit(10).unwrap() as usize;
                self.count = Some(self.count.unwrap_or(0) * 10 + digit);
            }

            // Escape — clear count, stay in normal
            KeyCode::Esc => {
                self.count = None;
                self.selection = None;
            }

            _ => {}
        }
    }

    // ─── Insert Mode ───────────────────────────────────────────────

    fn handle_insert(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                // Move cursor back one if possible (vim behavior)
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            KeyCode::Char(ch) => {
                self.buffer
                    .insert_char(self.cursor.row, self.cursor.col, ch);
                self.cursor.col += 1;
            }
            KeyCode::Enter => {
                let col = self.cursor.col;
                self.buffer.split_line(self.cursor.row, col);
                self.cursor.row += 1;
                self.cursor.col = 0;
            }
            KeyCode::Backspace => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                    self.buffer.delete_char(self.cursor.row, self.cursor.col);
                } else if self.cursor.row > 0 {
                    let prev_len = self.buffer.line_len(self.cursor.row - 1);
                    self.buffer.join_lines(self.cursor.row - 1);
                    self.cursor.row -= 1;
                    self.cursor.col = prev_len;
                }
            }
            KeyCode::Delete => {
                if self.cursor.col < self.buffer.line_len(self.cursor.row) {
                    self.buffer.delete_char(self.cursor.row, self.cursor.col);
                } else if self.cursor.row + 1 < self.buffer.line_count() {
                    self.buffer.join_lines(self.cursor.row);
                }
            }
            KeyCode::Tab => {
                // Insert 4 spaces (configurable in a real editor)
                for _ in 0..4 {
                    self.buffer
                        .insert_char(self.cursor.row, self.cursor.col, ' ');
                    self.cursor.col += 1;
                }
            }
            _ => {}
        }
    }

    // ─── Command Mode ──────────────────────────────────────────────

    fn handle_command(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.command_input.clear();
            }
            KeyCode::Enter => {
                let cmd = self.command_input.clone();
                self.mode = Mode::Normal;
                self.command_input.clear();
                self.execute_command(&cmd);
            }
            KeyCode::Backspace => {
                if self.command_cursor > 0 {
                    self.command_cursor -= 1;
                    self.command_input.remove(self.command_cursor);
                } else {
                    // Cancel command mode
                    self.mode = Mode::Normal;
                    self.command_input.clear();
                }
            }
            KeyCode::Char(ch) => {
                self.command_input.insert(self.command_cursor, ch);
                self.command_cursor += 1;
            }
            KeyCode::Left => {
                if self.command_cursor > 0 {
                    self.command_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.command_cursor < self.command_input.len() {
                    self.command_cursor += 1;
                }
            }
            _ => {}
        }
    }

    // ─── Replace Mode ──────────────────────────────────────────────

    fn handle_replace(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Char(ch) => {
                let len = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < len {
                    self.buffer.replace_range(
                        self.cursor.row,
                        self.cursor.col,
                        self.cursor.col + 1,
                        &ch.to_string(),
                    );
                    if self.cursor.col < self.buffer.line_len(self.cursor.row) - 1 {
                        self.cursor.col += 1;
                    }
                } else {
                    self.buffer
                        .insert_char(self.cursor.row, self.cursor.col, ch);
                    self.cursor.col += 1;
                }
            }
            KeyCode::Enter => {
                let col = self.cursor.col;
                self.buffer.split_line(self.cursor.row, col);
                self.cursor.row += 1;
                self.cursor.col = 0;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_editor(text: &str) -> Editor {
        let mut buf = Buffer::new();
        buf.insert_str(0, 0, text);
        Editor::new(buf)
    }

    fn make_multiline_editor(lines: &[&str]) -> Editor {
        let mut buf = Buffer::new();
        for (i, line) in lines.iter().enumerate() {
            if i == 0 {
                buf.replace_range(0, 0, 0, line);
            } else {
                buf.insert_line(i, line.to_string());
            }
        }
        Editor::new(buf)
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::char(c)
    }

    fn key_code(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::EMPTY)
    }

    fn esc() -> KeyEvent {
        key_code(KeyCode::Esc)
    }

    fn enter() -> KeyEvent {
        key_code(KeyCode::Enter)
    }

    fn backspace() -> KeyEvent {
        key_code(KeyCode::Backspace)
    }

    #[test]
    fn test_initial_state() {
        let ed = make_editor("hello");
        assert_eq!(ed.mode(), Mode::Normal);
        assert_eq!(ed.cursor().row, 0);
        assert_eq!(ed.cursor().col, 0);
        assert!(!ed.should_quit());
    }

    #[test]
    fn test_hjkl_movement() {
        let ed = make_multiline_editor(&["abc", "def", "ghi"]);
        let mut ed = ed;

        // j moves down
        ed.handle_key(key('j'));
        assert_eq!(ed.cursor().row, 1);

        // k moves up
        ed.handle_key(key('k'));
        assert_eq!(ed.cursor().row, 0);

        // l moves right
        ed.handle_key(key('l'));
        assert_eq!(ed.cursor().col, 1);

        // h moves left
        ed.handle_key(key('h'));
        assert_eq!(ed.cursor().col, 0);
    }

    #[test]
    fn test_insert_mode() {
        let mut ed = make_editor("");

        ed.handle_key(key('i'));
        assert_eq!(ed.mode(), Mode::Insert);

        ed.handle_key(key('h'));
        ed.handle_key(key('i'));
        assert_eq!(ed.buffer().line(0), "hi");

        ed.handle_key(esc());
        assert_eq!(ed.mode(), Mode::Normal);
    }

    #[test]
    fn test_insert_newline() {
        let mut ed = make_editor("hello world");
        // Move to col 5 (space between hello and world) using l in normal mode
        for _ in 0..5 {
            ed.handle_key(key('l'));
        }
        assert_eq!(ed.cursor().col, 5);

        ed.handle_key(key('i'));
        ed.handle_key(enter());

        assert_eq!(ed.buffer().line_count(), 2);
        assert_eq!(ed.buffer().line(0), "hello");
        assert_eq!(ed.buffer().line(1), " world");
    }

    #[test]
    fn test_delete_line_dd() {
        let mut ed = make_multiline_editor(&["line1", "line2", "line3"]);

        ed.handle_key(key('d'));

        assert_eq!(ed.buffer().line_count(), 2);
        assert_eq!(ed.buffer().line(0), "line2");
        assert_eq!(ed.buffer().line(1), "line3");
    }

    #[test]
    fn test_open_line_below() {
        let mut ed = make_editor("hello");

        ed.handle_key(key('o'));
        assert_eq!(ed.mode(), Mode::Insert);
        assert_eq!(ed.buffer().line_count(), 2);
        assert_eq!(ed.cursor().row, 1);
        assert_eq!(ed.cursor().col, 0);
    }

    #[test]
    fn test_open_line_above() {
        let mut ed = make_editor("hello");

        ed.handle_key(key('O'));
        assert_eq!(ed.mode(), Mode::Insert);
        assert_eq!(ed.buffer().line_count(), 2);
        assert_eq!(ed.cursor().row, 0);
        assert_eq!(ed.buffer().line(1), "hello");
    }

    #[test]
    fn test_command_mode_quit() {
        let mut ed = make_editor("");

        ed.handle_key(key(':'));
        assert_eq!(ed.mode(), Mode::Command);

        ed.handle_key(key('q'));
        ed.handle_key(enter());
        assert!(ed.should_quit());
    }

    #[test]
    fn test_command_mode_not_found() {
        let mut ed = make_editor("");

        ed.handle_key(key(':'));
        ed.handle_key(key('x'));
        ed.handle_key(key('y'));
        ed.handle_key(key('z'));
        ed.handle_key(enter());
        assert!(!ed.should_quit());
        assert!(ed.message().is_some());
    }

    #[test]
    fn test_visual_mode() {
        let mut ed = make_editor("hello");

        ed.handle_key(key('v'));
        assert_eq!(ed.mode(), Mode::Visual);

        ed.handle_key(esc());
        assert_eq!(ed.mode(), Mode::Normal);
    }

    #[test]
    fn test_append_at_end() {
        let mut ed = make_editor("ab");

        ed.handle_key(key('A'));
        assert_eq!(ed.mode(), Mode::Insert);
        assert_eq!(ed.cursor().col, 2);

        ed.handle_key(key('c'));
        assert_eq!(ed.buffer().line(0), "abc");
    }

    #[test]
    fn test_replace_mode() {
        let mut ed = make_editor("hello");

        ed.handle_key(key('R'));
        assert_eq!(ed.mode(), Mode::Replace);

        ed.handle_key(key('X'));
        assert_eq!(ed.buffer().line(0), "Xello");

        ed.handle_key(esc());
        assert_eq!(ed.mode(), Mode::Normal);
    }

    #[test]
    fn test_count_prefix() {
        let mut ed = make_editor("abcde");

        ed.handle_key(key('l')); // move to col 1
        ed.handle_key(key('l')); // move to col 2
                                 // Type 2 then 'x' to delete 2 chars
        ed.handle_key(key('2'));
        ed.handle_key(key('x'));
        assert_eq!(ed.buffer().line(0), "abe");
    }

    #[test]
    fn test_line_bol_eol() {
        let mut ed = make_editor("  hello");

        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        assert_eq!(ed.cursor().col, 5);

        // ^ goes to first non-blank
        ed.handle_key(key('^'));
        assert_eq!(ed.cursor().col, 2);

        // 0 goes to column 0
        ed.handle_key(key('0'));
        assert_eq!(ed.cursor().col, 0);

        // $ goes to end of line
        ed.handle_key(key('$'));
        assert_eq!(ed.cursor().col, 6);
    }

    #[test]
    fn test_join_lines() {
        let mut ed = make_multiline_editor(&["hello", "world"]);

        ed.handle_key(key('J'));
        assert_eq!(ed.buffer().line_count(), 1);
        assert_eq!(ed.buffer().line(0), "helloworld");
    }

    #[test]
    fn test_yank_and_paste() {
        let mut ed = make_multiline_editor(&["line1", "line2", "line3"]);

        // yank current line (line1)
        ed.handle_key(key('y'));
        // paste below
        ed.handle_key(key('p'));

        assert_eq!(ed.buffer().line_count(), 4);
        assert_eq!(ed.buffer().line(0), "line1");
        assert_eq!(ed.buffer().line(1), "line1");
    }

    #[test]
    fn test_backspace_in_insert() {
        let mut ed = make_editor("hello");
        ed.handle_key(key('A')); // append at end
        ed.handle_key(backspace());
        ed.handle_key(backspace());
        assert_eq!(ed.buffer().line(0), "hel");
    }

    #[test]
    fn test_quit_modified_buffer() {
        let mut ed = make_editor("");
        ed.handle_key(key('i'));
        ed.handle_key(key('x'));
        ed.handle_key(esc());

        ed.handle_key(key(':'));
        ed.handle_key(key('q'));
        ed.handle_key(enter());

        // Should NOT quit because buffer is modified
        assert!(!ed.should_quit());
        assert!(ed.message().unwrap().contains("No write"));
    }

    #[test]
    fn test_word_motion() {
        let mut ed = make_editor("hello world foo");

        ed.handle_key(key('w'));
        assert_eq!(ed.cursor().col, 6); // start of "world"

        ed.handle_key(key('w'));
        assert_eq!(ed.cursor().col, 12); // start of "foo"

        ed.handle_key(key('b'));
        assert_eq!(ed.cursor().col, 6); // back to "world"

        ed.handle_key(key('b'));
        assert_eq!(ed.cursor().col, 0); // back to "hello"
    }

    #[test]
    fn test_delete_char_x() {
        let mut ed = make_editor("hello");

        ed.handle_key(key('x'));
        assert_eq!(ed.buffer().line(0), "ello");

        ed.handle_key(key('x'));
        assert_eq!(ed.buffer().line(0), "llo");
    }

    #[test]
    fn test_go_to_end_G() {
        let mut ed = make_multiline_editor(&["a", "b", "c", "d"]);

        ed.handle_key(key('G'));
        assert_eq!(ed.cursor().row, 3);
    }

    #[test]
    fn test_go_to_top_g() {
        let mut ed = make_multiline_editor(&["a", "b", "c"]);
        ed.handle_key(key('G')); // go to end first

        ed.handle_key(key('g'));
        assert_eq!(ed.cursor().row, 0);
    }

    // ── Visual Mode Tests ─────────────────────────────────────────

    #[test]
    fn test_visual_mode_enter_exit() {
        let mut ed = make_editor("hello");
        ed.handle_key(key('v'));
        assert_eq!(ed.mode(), Mode::Visual);
        assert!(ed.selection().is_some());

        ed.handle_key(esc());
        assert_eq!(ed.mode(), Mode::Normal);
        assert!(ed.selection().is_none());
    }

    #[test]
    fn test_visual_line_enter_exit() {
        let mut ed = make_editor("hello");
        ed.handle_key(key('V'));
        assert_eq!(ed.mode(), Mode::VisualLine);

        let sel = ed.selection().unwrap();
        assert_eq!(sel.mode, SelectionMode::Line);

        ed.handle_key(esc());
        assert_eq!(ed.mode(), Mode::Normal);
    }

    #[test]
    fn test_visual_selection_grows() {
        let mut ed = make_editor("hello world");
        ed.handle_key(key('v'));
        // At col 0, selecting just 'h'
        let sel = ed.selection().unwrap();
        assert_eq!(sel.anchor_col, 0);
        assert_eq!(sel.head_col, 0);

        // Move right — selection grows
        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        ed.handle_key(key('l'));
        let sel = ed.selection().unwrap();
        assert_eq!(sel.head_col, 3);
        assert_eq!(sel.anchor_col, 0);
    }

    #[test]
    fn test_visual_delete_single_line() {
        let mut ed = make_editor("hello world");
        // Select "hello" (col 0-4)
        ed.handle_key(key('v'));
        for _ in 0..4 {
            ed.handle_key(key('l'));
        }
        // Now anchor=0, head=4, mode=Char
        ed.handle_key(key('d'));

        assert_eq!(ed.mode(), Mode::Normal);
        assert_eq!(ed.buffer().line(0), " world");
    }

    #[test]
    fn test_visual_yank_and_paste() {
        let mut ed = make_editor("hello world");
        // Select "hello"
        ed.handle_key(key('v'));
        for _ in 0..4 {
            ed.handle_key(key('l'));
        }
        ed.handle_key(key('y'));
        assert_eq!(ed.mode(), Mode::Normal);

        // Move to end and paste
        ed.handle_key(key('$'));
        ed.handle_key(key('p'));
        assert_eq!(ed.buffer().line(0), "hello worldhello");
    }

    #[test]
    fn test_visual_line_delete() {
        let mut ed = make_multiline_editor(&["line1", "line2", "line3"]);
        // Select first line (V selects one line)
        ed.handle_key(key('V'));
        // Move down to select second line too
        ed.handle_key(key('j'));
        // Delete both
        ed.handle_key(key('d'));

        assert_eq!(ed.mode(), Mode::Normal);
        assert_eq!(ed.buffer().line_count(), 1);
        assert_eq!(ed.buffer().line(0), "line3");
    }

    #[test]
    fn test_visual_toggle_anchor() {
        let mut ed = make_editor("hello world");
        ed.handle_key(key('v'));
        for _ in 0..4 {
            ed.handle_key(key('l'));
        }
        // Anchor at 0, head at 4
        let sel = ed.selection().unwrap();
        assert_eq!(sel.anchor_col, 0);
        assert_eq!(sel.head_col, 4);

        // Toggle with 'o'
        ed.handle_key(key('o'));
        let sel = ed.selection().unwrap();
        assert_eq!(sel.anchor_col, 4);
        assert_eq!(sel.head_col, 0);
    }

    #[test]
    fn test_visual_command_prefix() {
        let mut ed = make_editor("hello");
        ed.handle_key(key('v'));
        ed.handle_key(key('l'));
        ed.handle_key(key(':'));
        assert_eq!(ed.mode(), Mode::Command);
        assert_eq!(ed.command_input(), "'<,'>");
    }

    #[test]
    fn test_visual_change() {
        let mut ed = make_editor("hello world");
        // Select "hello"
        ed.handle_key(key('v'));
        for _ in 0..4 {
            ed.handle_key(key('l'));
        }
        ed.handle_key(key('c'));

        // Should be in insert mode with "hello" deleted
        assert_eq!(ed.mode(), Mode::Insert);
        assert_eq!(ed.buffer().line(0), " world");
    }
}
