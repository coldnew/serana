use display_protocol::{KeyCode, KeyEvent, KeyModifiers, Selection, SelectionMode};

use crate::buffer::Buffer;
use crate::cursor::Cursor;
use crate::mode::Mode;

mod commands;
mod motion;
mod visual;

const SCROLL_MARGIN: usize = 3;
const HORIZONTAL_SCROLL_MARGIN: usize = 5;

/// Normal-mode operator that can precede a motion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperatorKind {
    Delete,
    Yank,
    Change,
}

/// The main editor state.
pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
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
    /// Pending normal-mode operator. `d`, `y`, `c` set this; the next
    /// motion (or repeated operator) completes it.
    pending_operator: Option<OperatorKind>,
    /// Pending `g` prefix (waiting for second key like `gg`, `gj`, etc.)
    pending_g: bool,
    /// Last search pattern (for `n` / `N`).
    last_search: Option<String>,
    /// Whether the last search was forward.
    last_search_forward: bool,
    /// Last `f`/`F`/`t`/`T` find: (char, forward, till_mode).
    last_find: Option<(char, bool, bool)>,
    /// Pending find char (waiting for second key from f/F/t/T).
    pending_find: bool,
    /// Direction of pending find (forward or backward).
    pending_find_forward: bool,
    /// Till mode of pending find (t/T vs f/F).
    pending_find_till: bool,
    /// Undo stack (most recent first).
    undo_stack: Vec<UndoState>,
    /// Redo stack (most recent first).
    redo_stack: Vec<UndoState>,
    /// When true, modifications don't create new snapshots (insert mode, etc.)
    in_change_group: bool,
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

/// A snapshot of buffer + cursor for undo/redo.
#[derive(Debug, Clone)]
struct UndoState {
    lines: Vec<String>,
    cursor: Cursor,
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
            pending_operator: None,
            pending_g: false,
            last_search: None,
            last_search_forward: true,
            last_find: None,
            pending_find: false,
            pending_find_forward: true,
            pending_find_till: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            in_change_group: false,
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

        if self.mode == Mode::Normal && self.pending_g {
            self.pending_g = false;
            match key.code {
                KeyCode::Char('g') => {
                    let n = self.count.take().unwrap_or(1);
                    self.cursor.row = (n.saturating_sub(1))
                        .min(self.buffer.line_count().saturating_sub(1));
                    self.set_cursor_col(0);
                    self.clamp_cursor();
                    self.update_scroll();
                    return;
                }
                _ => {
                    self.count = None;
                }
            }
        }

        if self.mode == Mode::Normal && self.pending_find {
            self.pending_find = false;
            if let KeyCode::Char(ch) = key.code {
                let forward = self.pending_find_forward;
                let till = self.pending_find_till;
                self.last_find = Some((ch, forward, till));
                self.find_char(ch, forward, till);
                self.clamp_cursor();
                self.update_scroll();
                return;
            }
        }

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

    /// Push an undo snapshot (save current state).
    fn push_undo_state(&mut self) {
        if self.in_change_group {
            return;
        }
        self.undo_stack.push(UndoState {
            lines: self.buffer.all_lines(),
            cursor: self.cursor,
        });
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Undo the last change.
    fn undo(&mut self) {
        let Some(state) = self.undo_stack.pop() else {
            self.message = Some("Already at oldest change".into());
            return;
        };
        self.redo_stack.push(UndoState {
            lines: self.buffer.all_lines(),
            cursor: self.cursor,
        });
        self.buffer.restore_lines(state.lines);
        self.cursor = state.cursor;
        self.cursor.preferred_col = self.cursor.col;
        self.message = None;
    }

    /// Redo the last undone change.
    fn redo(&mut self) {
        let Some(state) = self.redo_stack.pop() else {
            self.message = Some("Already at newest change".into());
            return;
        };
        self.undo_stack.push(UndoState {
            lines: self.buffer.all_lines(),
            cursor: self.cursor,
        });
        self.buffer.restore_lines(state.lines);
        self.cursor = state.cursor;
        self.cursor.preferred_col = self.cursor.col;
        self.message = None;
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

        if text_height == 0 || text_width == 0 {
            return;
        }

        let line_count = self.buffer.line_count();

        // Vertical scroll
        let scroll_margin = SCROLL_MARGIN.min(text_height.saturating_sub(1) / 2);
        if self.cursor.row < self.scroll.row + scroll_margin {
            self.scroll.row = self.cursor.row.saturating_sub(scroll_margin);
        } else if self.cursor.row + scroll_margin >= self.scroll.row + text_height {
            self.scroll.row = (self.cursor.row + scroll_margin + 1).saturating_sub(text_height);
        }
        let max_scroll_row = line_count.saturating_sub(text_height);
        self.scroll.row = self.scroll.row.min(max_scroll_row);

        // Ensure cursor is always on a visible content row
        if self.cursor.row < self.scroll.row {
            self.scroll.row = self.cursor.row;
        } else if line_count > 0 && self.cursor.row >= self.scroll.row + text_height {
            self.scroll.row = self.cursor.row.saturating_sub(text_height - 1);
            self.scroll.row = self.scroll.row.min(max_scroll_row);
        }

        // Horizontal scroll
        let col_margin = HORIZONTAL_SCROLL_MARGIN.min(text_width.saturating_sub(1) / 2);
        if self.cursor.col < self.scroll.col + col_margin {
            self.scroll.col = self.cursor.col.saturating_sub(col_margin);
        } else if self.cursor.col + col_margin >= self.scroll.col + text_width {
            self.scroll.col = (self.cursor.col + col_margin + 1).saturating_sub(text_width);
        }
    }

    // ─── Normal Mode ───────────────────────────────────────────────

    fn handle_normal(&mut self, key: KeyEvent) {
        if let Some(op) = self.pending_operator {
            self.execute_operator_motion(op, key);
            return;
        }

        if key.modifiers.ctrl && self.handle_ctrl_normal(key) {
            return;
        }

        match key.code {
            // Movement
            KeyCode::Char('h') | KeyCode::Left => {
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col > 0 {
                        self.set_cursor_col(self.cursor.col - 1);
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                let n = self.take_count();
                for _ in 0..n {
                    self.move_cursor_down(1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let n = self.take_count();
                for _ in 0..n {
                    self.move_cursor_up(1);
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                let n = self.take_count();
                for _ in 0..n {
                    let max_col = self.buffer.line_len(self.cursor.row).saturating_sub(1);
                    if self.cursor.col < max_col {
                        self.set_cursor_col(self.cursor.col + 1);
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
            KeyCode::Char('$') => {
                let len = self.buffer.line_len(self.cursor.row);
                self.set_cursor_col(if len > 0 { len - 1 } else { 0 });
            }
            KeyCode::Char('^') => {
                // First non-blank character
                let line = self.buffer.line(self.cursor.row);
                self.set_cursor_col(line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0));
            }
            KeyCode::Char('G') => {
                let last = self.buffer.line_count().saturating_sub(1);
                if self.count.is_some() {
                    let line = self.take_count();
                    self.cursor.row = (line.saturating_sub(1)).min(last);
                } else {
                    self.cursor.row = last;
                }
                self.set_cursor_col(0);
                self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
            }
            KeyCode::Char('g') => {
                self.pending_g = true;
            }

            // Screen-relative motion
            KeyCode::Char('H') => {
                // Top of screen
                self.cursor.row = self.scroll.row;
                self.apply_preferred_col();
            }
            KeyCode::Char('L') => {
                // Bottom of screen
                let text_height = (self.term_height as usize).saturating_sub(2);
                let bottom = (self.scroll.row + text_height - 1)
                    .min(self.buffer.line_count().saturating_sub(1));
                self.cursor.row = bottom;
                self.apply_preferred_col();
            }
            KeyCode::Char('M') => {
                // Middle of screen
                let text_height = (self.term_height as usize).saturating_sub(2);
                let mid = self.scroll.row + text_height / 2;
                self.cursor.row = mid.min(self.buffer.line_count().saturating_sub(1));
                self.apply_preferred_col();
            }

            // Page up/down
            KeyCode::PageDown | KeyCode::Char('\x06') => {
                let text_height = (self.term_height as usize).saturating_sub(2);
                let n = self.take_count();
                for _ in 0..n {
                    self.move_cursor_down(text_height);
                }
            }
            KeyCode::PageUp | KeyCode::Char('\x02') => {
                let text_height = (self.term_height as usize).saturating_sub(2);
                let n = self.take_count();
                for _ in 0..n {
                    self.move_cursor_up(text_height);
                }
            }

            // Enter insert mode
            KeyCode::Char('i') => {
                self.push_undo_state();
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('I') => {
                self.push_undo_state();
                let line = self.buffer.line(self.cursor.row);
                self.set_cursor_col(line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0));
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('a') => {
                self.push_undo_state();
                let max_col = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < max_col {
                    self.set_cursor_col(self.cursor.col + 1);
                }
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('A') => {
                self.push_undo_state();
                self.set_cursor_col(self.buffer.line_len(self.cursor.row));
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('o') => {
                self.push_undo_state();
                let new_row = self.cursor.row + 1;
                self.buffer.insert_line(new_row, String::new());
                self.cursor.row = new_row;
                self.set_cursor_col(0);
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('O') => {
                self.push_undo_state();
                self.buffer.insert_line(self.cursor.row, String::new());
                self.set_cursor_col(0);
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }

            // Delete operations
            KeyCode::Char('x') => {
                self.push_undo_state();
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col < self.buffer.line_len(self.cursor.row) {
                        self.buffer.delete_char(self.cursor.row, self.cursor.col);
                    }
                }
            }
            KeyCode::Char('X') => {
                self.push_undo_state();
                let n = self.take_count();
                for _ in 0..n {
                    if self.cursor.col > 0 {
                        self.cursor.col -= 1;
                        self.buffer.delete_char(self.cursor.row, self.cursor.col);
                    }
                }
            }
            KeyCode::Char('d') => {
                self.pending_operator = Some(OperatorKind::Delete);
            }
            KeyCode::Char('D') => {
                self.push_undo_state();
                let len = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < len {
                    self.buffer
                        .replace_range(self.cursor.row, self.cursor.col, len, "");
                }
            }
            KeyCode::Char('c') => {
                self.pending_operator = Some(OperatorKind::Change);
            }
            KeyCode::Char('C') => {
                self.push_undo_state();
                let len = self.buffer.line_len(self.cursor.row);
                if self.cursor.col < len {
                    self.buffer
                        .replace_range(self.cursor.row, self.cursor.col, len, "");
                }
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('S') => {
                self.push_undo_state();
                let _ = self.buffer.delete_line(self.cursor.row);
                self.buffer.insert_line(self.cursor.row, String::new());
                self.set_cursor_col(0);
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
            KeyCode::Char('s') => {
                self.push_undo_state();
                if self.cursor.col < self.buffer.line_len(self.cursor.row) {
                    self.buffer.delete_char(self.cursor.row, self.cursor.col);
                }
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }

            // Yank — wait for y or a motion
            KeyCode::Char('y') => {
                self.pending_operator = Some(OperatorKind::Yank);
            }
            KeyCode::Char('Y') => {
                let line = self.buffer.line(self.cursor.row).to_string();
                self.yank_register = YankRegister::Chars(line);
                self.message = Some("1 line yanked".into());
            }
            KeyCode::Char('p') => {
                let n = self.take_count();
                match &self.yank_register {
                    YankRegister::Lines(lines) => {
                        for _ in 0..n {
                            for (i, line) in lines.iter().enumerate() {
                                self.buffer
                                    .insert_line(self.cursor.row + 1 + i, line.clone());
                            }
                        }
                        self.cursor.row += 1;
                        self.set_cursor_col(0);
                    }
                    YankRegister::Chars(text) => {
                        let text = text.clone();
                        let col = self.cursor.col + 1;
                        for _ in 0..n {
                            self.buffer.insert_str(self.cursor.row, col, &text);
                        }
                    }
                }
            }
            KeyCode::Char('P') => {
                let n = self.take_count();
                match &self.yank_register {
                    YankRegister::Lines(lines) => {
                        for _ in 0..n {
                            for (i, line) in lines.iter().enumerate() {
                                self.buffer.insert_line(self.cursor.row + i, line.clone());
                            }
                        }
                        self.set_cursor_col(0);
                    }
                    YankRegister::Chars(text) => {
                        let text = text.clone();
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

            // Undo
            KeyCode::Char('u') => {
                self.undo();
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
                self.push_undo_state();
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

            // Search repeat
            KeyCode::Char('n') => {
                self.repeat_search(true);
            }
            KeyCode::Char('N') => {
                self.repeat_search(false);
            }

            // Word-under-cursor search
            KeyCode::Char('*') => {
                self.search_word_under_cursor(true);
            }
            KeyCode::Char('#') => {
                self.search_word_under_cursor(false);
            }

            // Bracket matching
            KeyCode::Char('%') => {
                self.jump_to_matching_bracket();
            }

            // Toggle case
            KeyCode::Char('~') => {
                self.push_undo_state();
                let line_len = self.buffer.line_len(self.cursor.row);
                let col = self.cursor.col;
                if col < line_len {
                    let ch = self.buffer.line(self.cursor.row).as_bytes()[col] as char;
                    let toggled = if ch.is_ascii_lowercase() {
                        ch.to_ascii_uppercase()
                    } else {
                        ch.to_ascii_lowercase()
                    };
                    self.buffer.replace_range(
                        self.cursor.row, col, col + 1, &toggled.to_string(),
                    );
                    if col + 1 < self.buffer.line_len(self.cursor.row) {
                        self.set_cursor_col(col + 1);
                    }
                }
            }

            // Line-relative movement (first non-blank)
            KeyCode::Enter | KeyCode::Char('+') => {
                let n = self.take_count();
                self.cursor.row = (self.cursor.row + n).min(self.buffer.line_count().saturating_sub(1));
                let line = self.buffer.line(self.cursor.row);
                self.set_cursor_col(line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0));
            }
            KeyCode::Char('-') => {
                let n = self.take_count();
                self.cursor.row = self.cursor.row.saturating_sub(n);
                let line = self.buffer.line(self.cursor.row);
                self.set_cursor_col(line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0));
            }
            KeyCode::Char('_') => {
                let n = self.take_count();
                let target = (self.cursor.row + n.saturating_sub(1))
                    .min(self.buffer.line_count().saturating_sub(1));
                self.cursor.row = target;
                let line = self.buffer.line(self.cursor.row);
                self.set_cursor_col(line.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0));
            }

            // Find/till on line
            KeyCode::Char('f') => {
                self.pending_find = true;
                self.pending_find_forward = true;
                self.pending_find_till = false;
            }
            KeyCode::Char('F') => {
                self.pending_find = true;
                self.pending_find_forward = false;
                self.pending_find_till = false;
            }
            KeyCode::Char('t') => {
                self.pending_find = true;
                self.pending_find_forward = true;
                self.pending_find_till = true;
            }
            KeyCode::Char('T') => {
                self.pending_find = true;
                self.pending_find_forward = false;
                self.pending_find_till = true;
            }
            KeyCode::Char(';') => {
                if let Some((ch, forward, till)) = self.last_find {
                    self.find_char(ch, forward, till);
                }
            }
            KeyCode::Char(',') => {
                if let Some((ch, forward, till)) = self.last_find {
                    self.find_char(ch, !forward, till);
                }
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
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if c == '0' && self.count.is_none() {
                    self.set_cursor_col(0);
                } else {
                    let digit = c.to_digit(10).unwrap() as usize;
                    self.count = Some(self.count.unwrap_or(0) * 10 + digit);
                }
            }

            // Escape — clear count/selection/pending
            KeyCode::Esc => {
                self.count = None;
                self.selection = None;
                self.pending_operator = None;
                self.pending_g = false;
                self.pending_find = false;
            }

            _ => {}
        }
    }

    /// Execute an operator (d/y/c) followed by a motion (second key).
    fn execute_operator_motion(&mut self, op: OperatorKind, key: KeyEvent) {
        // Consume the pending operator
        self.pending_operator = None;

        // Handle line-wise operator (dd, yy, cc)
        match key.code {
            KeyCode::Char('d') | KeyCode::Char('y') | KeyCode::Char('c') => {
                let n = self.take_count();
                match op {
                    OperatorKind::Delete => {
                        self.push_undo_state();
                        self.delete_n_lines(n);
                    }
                    OperatorKind::Yank => self.yank_n_lines(n),
                    OperatorKind::Change => {
                        self.push_undo_state();
                        self.in_change_group = true;
                        self.change_n_lines(n);
                    }
                }
                return;
            }
            _ => {}
        }

        // For operator + motion, save cursor and apply motion
        // Then apply operation over the range [saved_cursor, cursor)
        let saved_row = self.cursor.row;
        let saved_col = self.cursor.col;
        let saved_scroll = self.scroll;

        // Apply the motion
        let prev_count = self.count.take();
        self.handle_normal(key);

        let end_row = self.cursor.row;
        let end_col = self.cursor.col;

        // Restore scroll (don't want the motion to scroll independently)
        self.scroll = saved_scroll;

        match op {
            OperatorKind::Delete => {
                self.push_undo_state();
                if end_row > saved_row || (end_row == saved_row && end_col > saved_col) {
                    // Forward motion: delete from saved to end
                    self.apply_delete_range((saved_row, saved_col), (end_row, end_col));
                    self.cursor.row = saved_row;
                    self.cursor.col = saved_col;
                } else if end_row < saved_row || (end_row == saved_row && end_col < saved_col) {
                    // Backward motion: delete from end to saved
                    self.apply_delete_range((end_row, end_col), (saved_row, saved_col));
                    self.cursor.row = end_row;
                    self.cursor.col = end_col;
                }
            }
            OperatorKind::Yank => {
                if end_row > saved_row || (end_row == saved_row && end_col > saved_col) {
                    let text = self.extract_range((saved_row, saved_col), (end_row, end_col));
                    self.yank_register = YankRegister::Chars(text);
                } else {
                    let text = self.extract_range((end_row, end_col), (saved_row, saved_col));
                    self.yank_register = YankRegister::Chars(text);
                }
                self.cursor.row = saved_row;
                self.cursor.col = saved_col;
            }
            OperatorKind::Change => {
                self.push_undo_state();
                if end_row > saved_row || (end_row == saved_row && end_col > saved_col) {
                    self.apply_delete_range((saved_row, saved_col), (end_row, end_col));
                    self.cursor.row = saved_row;
                    self.cursor.col = saved_col;
                } else {
                    self.apply_delete_range((end_row, end_col), (saved_row, saved_col));
                    self.cursor.row = end_row;
                    self.cursor.col = end_col;
                }
                self.in_change_group = true;
                self.mode = Mode::Insert;
            }
        }

        // Restore count if consumed
        if let Some(c) = prev_count {
            self.count = Some(c);
        }
    }

    fn handle_ctrl_normal(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('f') => { self.move_pages_down(); true }
            KeyCode::Char('b') => { self.move_pages_up(); true }
            KeyCode::Char('d') => { self.move_half_pages_down(); true }
            KeyCode::Char('u') => { self.move_half_pages_up(); true }
            KeyCode::Char('e') => { self.scroll_one_line_down(); true }
            KeyCode::Char('y') => { self.scroll_one_line_up(); true }
            KeyCode::Char('a') => { self.increment_number(true); true }
            KeyCode::Char('x') => { self.increment_number(false); true }
            KeyCode::Char('r') => { self.redo(); true }
            _ => false,
        }
    }

    fn delete_n_lines(&mut self, n: usize) {
        let mut lines = Vec::new();
        let start_row = self.cursor.row;
        for _ in 0..n {
            if let Some(line) = self.buffer.delete_line(self.cursor.row) {
                lines.push(line);
            }
        }
        if !lines.is_empty() {
            self.yank_register = YankRegister::Lines(lines);
        }
        self.cursor.row = start_row.min(self.buffer.line_count().saturating_sub(1));
        self.cursor.clamp_col(self.buffer.line_len(self.cursor.row));
        self.cursor.preferred_col = self.cursor.col;
    }

    fn yank_n_lines(&mut self, n: usize) {
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

    fn change_n_lines(&mut self, n: usize) {
        self.delete_n_lines(n);
        self.buffer.insert_line(self.cursor.row, String::new());
        self.set_cursor_col(0);
        self.mode = Mode::Insert;
    }

    fn move_pages_down(&mut self) {
        let text_height = (self.term_height as usize).saturating_sub(2);
        let n = self.take_count();
        for _ in 0..n {
            self.move_cursor_down(text_height);
        }
    }

    fn move_pages_up(&mut self) {
        let text_height = (self.term_height as usize).saturating_sub(2);
        let n = self.take_count();
        for _ in 0..n {
            self.move_cursor_up(text_height);
        }
    }

    fn move_half_pages_down(&mut self) {
        let half_page = (self.term_height as usize).saturating_sub(2).max(1) / 2;
        let n = self.take_count();
        for _ in 0..n {
            self.move_cursor_down(half_page.max(1));
        }
    }

    fn move_half_pages_up(&mut self) {
        let half_page = (self.term_height as usize).saturating_sub(2).max(1) / 2;
        let n = self.take_count();
        for _ in 0..n {
            self.move_cursor_up(half_page.max(1));
        }
    }

    fn set_cursor_col(&mut self, col: usize) {
        self.cursor.col = col;
        self.cursor.preferred_col = col;
    }

    fn apply_preferred_col(&mut self) {
        self.cursor.col = self
            .cursor
            .preferred_col
            .min(self.buffer.line_len(self.cursor.row));
    }

    fn move_cursor_down(&mut self, lines: usize) {
        self.cursor.row = (self.cursor.row + lines).min(self.buffer.line_count().saturating_sub(1));
        self.apply_preferred_col();
    }

    fn move_cursor_up(&mut self, lines: usize) {
        self.cursor.row = self.cursor.row.saturating_sub(lines);
        self.apply_preferred_col();
    }

    fn scroll_one_line_down(&mut self) {
        let text_height = (self.term_height as usize).saturating_sub(2);
        if self.scroll.row + 1 + text_height <= self.buffer.line_count() {
            self.scroll.row += 1;
        }
        self.cursor.row = self.cursor.row.min(self.scroll.row + text_height - 1);
        self.cursor.row = self.cursor.row.max(self.scroll.row);
    }

    fn scroll_one_line_up(&mut self) {
        if self.scroll.row > 0 {
            self.scroll.row -= 1;
        }
        self.cursor.row = self.cursor.row.max(self.scroll.row);
        self.cursor.row = self.cursor.row.min(self.scroll.row + (self.term_height as usize).saturating_sub(2) - 1);
    }

    /// Repeat last search (`n` / `N`).
    fn repeat_search(&mut self, forward: bool) {
        let Some(ref pattern) = self.last_search.clone() else {
            self.message = Some("No previous search".into());
            return;
        };
        let dir = if forward { 1 } else { -1 };
        self.search_pattern(&pattern, dir);
    }

    /// Search for word under cursor (`*` / `#`).
    fn search_word_under_cursor(&mut self, forward: bool) {
        let line = self.buffer.line(self.cursor.row);
        if line.is_empty() {
            return;
        }
        let word = extract_word_at(line, self.cursor.col);
        self.last_search = Some(word.clone());
        self.last_search_forward = forward;
        let dir = if forward { 1 } else { -1 };
        self.search_pattern(&word, dir);
    }

    fn search_pattern(&mut self, pattern: &str, dir: isize) {
        let start = self.cursor.row as isize;
        let total = self.buffer.line_count() as isize;
        let mut row = start + dir;
        let mut found = false;

        while row >= 0 && row < total {
            if self.buffer.line(row as usize).contains(pattern) {
                self.cursor.row = row as usize;
                self.set_cursor_col(
                    self.buffer.line(row as usize).find(pattern).unwrap_or(0),
                );
                found = true;
                break;
            }
            row += dir;
        }

        if !found {
            self.message = Some(format!("Pattern not found: {}", pattern));
        }
    }

    /// Jump to matching bracket `()`, `[]`, `{}`.
    fn jump_to_matching_bracket(&mut self) {
        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();
        let col = self.cursor.col;

        let (open, close) = match bytes.get(col) {
            Some(b'(') => (b'(', b')'),
            Some(b')') => (b')', b'('),
            Some(b'[') => (b'[', b']'),
            Some(b']') => (b']', b'['),
            Some(b'{') => (b'{', b'}'),
            Some(b'}') => (b'}', b'{'),
            _ => return,
        };

        let is_forward = open == b'(' || open == b'[' || open == b'{';
        let dir: isize = if is_forward { 1 } else { -1 };
        let target = if is_forward { close } else { open };
        let mut depth = 1;
        let mut r = self.cursor.row as isize;
        let mut c = col as isize + dir;

        loop {
            if c < 0 || c as usize >= self.buffer.line_len(r as usize) {
                r += dir;
                if r < 0 || r as usize >= self.buffer.line_count() {
                    self.message = Some("Unmatched bracket".into());
                    return;
                }
                c = if is_forward { 0 } else { (self.buffer.line_len(r as usize) as isize) - 1 };
                continue;
            }

            let ch = self.buffer.line(r as usize).as_bytes()[c as usize];
            if ch == open || ch == close {
                if ch == target {
                    depth -= 1;
                    if depth == 0 {
                        self.cursor.row = r as usize;
                        self.cursor.col = c as usize;
                        return;
                    }
                } else {
                    depth += 1;
                }
            }
            c += dir;
        }
    }

    /// Find char on current line (f/F/t/T).
    fn find_char(&mut self, ch: char, forward: bool, till: bool) {
        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();
        let col = self.cursor.col;
        let target = ch as u8;

        if forward {
            let mut c = col + 1;
            while c < bytes.len() {
                if bytes[c] == target {
                    self.cursor.col = if till { c } else { c };
                    self.cursor.preferred_col = self.cursor.col;
                    return;
                }
                c += 1;
            }
        } else {
            let mut c = col.saturating_sub(1);
            loop {
                if bytes[c] == target {
                    self.cursor.col = if till { c + 1 } else { c };
                    self.cursor.preferred_col = self.cursor.col;
                    return;
                }
                if c == 0 {
                    break;
                }
                c -= 1;
            }
        }
    }

    /// Increment or decrement the number under the cursor (Ctrl+A / Ctrl+X).
    fn increment_number(&mut self, add: bool) {
        let col = self.cursor.col;
        let line_len = self.buffer.line_len(self.cursor.row);
        if line_len == 0 {
            return;
        }
        let line = self.buffer.line(self.cursor.row).to_string();
        let bytes = line.as_bytes();

        let mut start = col;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        let mut end = col;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if start >= end {
            return;
        }

        let num_str = line[start..end].to_string();
        if let Ok(mut n) = num_str.parse::<i64>() {
            let count = self.take_count() as i64;
            if add {
                n += count;
            } else {
                n -= count;
            }
            let padded = format!("{:0>width$}", n, width = num_str.len());
            self.buffer.replace_range(self.cursor.row, start, end, &padded);
            self.cursor.col = start;
        }
    }

    // ─── Range helpers for operator+motion ─────────────────────────

    fn apply_delete_range(&mut self, from: (usize, usize), to: (usize, usize)) {
        let (sr, sc) = from;
        let (er, ec) = to;
        if sr == er {
            let text = self.buffer.line(sr)[sc..ec].to_string();
            self.buffer.replace_range(sr, sc, ec, "");
            self.yank_register = YankRegister::Chars(text);
        } else {
            let mut parts = Vec::new();
            parts.push(self.buffer.line(sr)[sc..].to_string());
            for i in (sr + 1)..er {
                parts.push(self.buffer.line(i).to_string());
            }
            if ec <= self.buffer.line_len(er) {
                parts.push(self.buffer.line(er)[..ec].to_string());
            } else {
                parts.push(self.buffer.line(er).to_string());
            }
            self.yank_register = YankRegister::Chars(parts.join("\n"));

            self.buffer.replace_range(sr, sc, self.buffer.line_len(sr), "");
            for _ in (sr + 1)..=er {
                if self.buffer.line_count() > sr + 1 {
                    self.buffer.delete_line(sr + 1);
                }
            }
            if sr + 1 < self.buffer.line_count() && ec <= self.buffer.line_len(sr + 1) {
                self.buffer.replace_range(sr + 1, 0, ec, "");
                self.buffer.join_lines(sr);
            }
        }
    }

    fn extract_range(&mut self, from: (usize, usize), to: (usize, usize)) -> String {
        let (sr, sc) = from;
        let (er, ec) = to;
        if sr == er {
            self.buffer.line(sr)[sc..ec].to_string()
        } else {
            let mut parts = Vec::new();
            parts.push(self.buffer.line(sr)[sc..].to_string());
            for i in (sr + 1)..er {
                parts.push(self.buffer.line(i).to_string());
            }
            if ec <= self.buffer.line_len(er) {
                parts.push(self.buffer.line(er)[..ec].to_string());
            } else {
                parts.push(self.buffer.line(er).to_string());
            }
            parts.join("\n")
        }
    }

    // ─── Insert Mode ───────────────────────────────────────────────

    fn handle_insert(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('\x1b') => {
                self.in_change_group = false;
                self.mode = Mode::Normal;
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
                self.cursor.preferred_col = self.cursor.col;
            }
            KeyCode::Char(c @ ('\x03' | '\x17' | '\x15' | '\x7f')) => {
                match c {
                    '\x03' => {
                        // Ctrl+C — exit insert
                        self.in_change_group = false;
                        self.mode = Mode::Normal;
                        self.cursor.preferred_col = self.cursor.col;
                    }
                    '\x17' => self.delete_word_before_cursor(), // Ctrl+W
                    '\x15' => {
                        // Ctrl+U — delete to start of line
                        let col = self.cursor.col;
                        if col > 0 {
                            self.buffer.replace_range(self.cursor.row, 0, col, "");
                            self.cursor.col = 0;
                            self.cursor.preferred_col = 0;
                        }
                    }
                    '\x7f' => {
                        // DEL (same as backspace)
                        if self.cursor.col > 0 {
                            self.set_cursor_col(self.cursor.col - 1);
                            self.buffer.delete_char(self.cursor.row, self.cursor.col);
                        } else if self.cursor.row > 0 {
                            let prev_len = self.buffer.line_len(self.cursor.row - 1);
                            self.buffer.join_lines(self.cursor.row - 1);
                            self.cursor.row -= 1;
                            self.set_cursor_col(prev_len);
                        }
                    }
                    _ => unreachable!(),
                }
            }
            KeyCode::Char(ch) => {
                self.buffer
                    .insert_char(self.cursor.row, self.cursor.col, ch);
                self.set_cursor_col(self.cursor.col + 1);
            }
            KeyCode::Enter => {
                let col = self.cursor.col;
                self.buffer.split_line(self.cursor.row, col);
                self.cursor.row += 1;
                self.set_cursor_col(0);
            }
            KeyCode::Backspace => {
                if self.cursor.col > 0 {
                    self.set_cursor_col(self.cursor.col - 1);
                    self.buffer.delete_char(self.cursor.row, self.cursor.col);
                } else if self.cursor.row > 0 {
                    let prev_len = self.buffer.line_len(self.cursor.row - 1);
                    self.buffer.join_lines(self.cursor.row - 1);
                    self.cursor.row -= 1;
                    self.set_cursor_col(prev_len);
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
                for _ in 0..4 {
                    self.buffer
                        .insert_char(self.cursor.row, self.cursor.col, ' ');
                    self.set_cursor_col(self.cursor.col + 1);
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
            KeyCode::Esc | KeyCode::Char('\x1b') | KeyCode::Char('\x03') => {
                self.in_change_group = false;
                self.mode = Mode::Normal;
                self.cursor.preferred_col = self.cursor.col;
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
                        self.set_cursor_col(self.cursor.col + 1);
                    }
                } else {
                    self.buffer
                        .insert_char(self.cursor.row, self.cursor.col, ch);
                    self.set_cursor_col(self.cursor.col + 1);
                }
            }
            KeyCode::Enter => {
                let col = self.cursor.col;
                self.buffer.split_line(self.cursor.row, col);
                self.cursor.row += 1;
                self.set_cursor_col(0);
            }
            _ => {}
        }
    }

    /// Delete the word before the cursor (for Ctrl+W in insert mode).
    fn delete_word_before_cursor(&mut self) {
        if self.cursor.col == 0 {
            return;
        }
        let line = self.buffer.line(self.cursor.row);
        let bytes = line.as_bytes();
        let end = self.cursor.col;
        // Skip spaces
        let mut start = end;
        while start > 0 && bytes[start - 1] == b' ' {
            start -= 1;
        }
        // Skip word chars
        while start > 0 && bytes[start - 1] != b' ' {
            start -= 1;
        }
        if start < end {
            self.buffer.replace_range(self.cursor.row, start, end, "");
            self.cursor.col = start;
            self.cursor.preferred_col = start;
        }
    }

    // ─── Visual selection operations ───────────────────────────────

    fn indent_selection(&mut self, amount: usize) {
        let Some(ref sel) = self.selection else { return };
        let (sl, _sc, el, _ec) = sel.normalized_range();
        let indent = " ".repeat(amount);
        for row in sl..=el {
            let r = row as usize;
            if r < self.buffer.line_count() {
                self.buffer.insert_str(r, 0, &indent);
            }
        }
        self.cursor.row = sl as usize;
        self.cursor.col = 0;
    }

    fn dedent_selection(&mut self, amount: usize) {
        let Some(ref sel) = self.selection else { return };
        let (sl, _sc, el, _ec) = sel.normalized_range();
        for row in sl..=el {
            let r = row as usize;
            if r < self.buffer.line_count() {
                let line = self.buffer.line(r);
                let to_remove = line.len().min(amount);
                if to_remove > 0 {
                    self.buffer.replace_range(r, 0, to_remove, "");
                }
            }
        }
        self.cursor.row = sl as usize;
        self.cursor.col = 0;
    }

    fn case_selection(&mut self, upper: bool) {
        let Some(ref sel) = self.selection else { return };
        let (sl, sc, el, ec) = sel.normalized_range();
        for row in sl..=el {
            let r = row as usize;
            if r >= self.buffer.line_count() {
                continue;
            }
            let line = self.buffer.line(r).to_string();
            let start_col = if r == sl as usize { sc as usize } else { 0 };
            let end_col = if r == el as usize { (ec as usize + 1).min(line.len()) } else { line.len() };
            if start_col >= end_col {
                continue;
            }
            let mut chars: Vec<char> = line[start_col..end_col].chars().collect();
            for ch in &mut chars {
                *ch = if upper { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() };
            }
            let new_part: String = chars.into_iter().collect();
            self.buffer.replace_range(r, start_col, end_col, &new_part);
        }
        self.cursor.row = sl as usize;
        self.cursor.col = sc as usize;
    }

    fn toggle_case_selection(&mut self) {
        let Some(ref sel) = self.selection else { return };
        let (sl, sc, el, ec) = sel.normalized_range();
        for row in sl..=el {
            let r = row as usize;
            if r >= self.buffer.line_count() {
                continue;
            }
            let line = self.buffer.line(r).to_string();
            let start_col = if r == sl as usize { sc as usize } else { 0 };
            let end_col = if r == el as usize { (ec as usize + 1).min(line.len()) } else { line.len() };
            if start_col >= end_col {
                continue;
            }
            let mut chars: Vec<char> = line[start_col..end_col].chars().collect();
            for ch in &mut chars {
                *ch = if ch.is_ascii_lowercase() {
                    ch.to_ascii_uppercase()
                } else {
                    ch.to_ascii_lowercase()
                };
            }
            let new_part: String = chars.into_iter().collect();
            self.buffer.replace_range(r, start_col, end_col, &new_part);
        }
        self.cursor.row = sl as usize;
        self.cursor.col = sc as usize;
    }
}

/// Extract the word (alphanumeric + _) at `col` in `line`.
fn extract_word_at(line: &str, col: usize) -> String {
    let bytes = line.as_bytes();
    if bytes.is_empty() {
        return String::new();
    }
    let mut start = col;
    while start > 0 && is_word_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_word_char(bytes[end]) {
        end += 1;
    }
    if start >= end {
        return String::new();
    }
    line[start..end].to_string()
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
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

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
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

    fn ex(ed: &mut Editor, command: &str) {
        ed.handle_key(key(':'));
        for ch in command.chars() {
            ed.handle_key(key(ch));
        }
        ed.handle_key(enter());
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
    fn test_vertical_movement_preserves_preferred_column() {
        let mut ed = make_multiline_editor(&["abcdef", "x", "abcdef"]);

        for _ in 0..5 {
            ed.handle_key(key('l'));
        }
        assert_eq!(ed.cursor().col, 5);

        ed.handle_key(key('j'));
        assert_eq!(ed.cursor().row, 1);
        assert_eq!(ed.cursor().col, 1);

        ed.handle_key(key('j'));
        assert_eq!(ed.cursor().row, 2);
        assert_eq!(ed.cursor().col, 5);
    }

    #[test]
    fn test_scroll_margin_moves_view_before_cursor_hits_edge() {
        let mut ed = make_multiline_editor(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
        ed.set_term_size(80, 6);

        ed.handle_key(key('j'));
        ed.handle_key(key('j'));
        assert_eq!(ed.scroll().row, 0);

        ed.handle_key(key('j'));
        assert_eq!(ed.cursor().row, 3);
        assert_eq!(ed.scroll().row, 1);
    }

    #[test]
    fn test_ctrl_f_and_ctrl_b_scroll_full_page() {
        let mut ed = make_multiline_editor(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
        ed.set_term_size(80, 6);

        ed.handle_key(ctrl_key('f'));
        assert_eq!(ed.cursor().row, 4);

        ed.handle_key(ctrl_key('b'));
        assert_eq!(ed.cursor().row, 0);
    }

    #[test]
    fn test_ctrl_d_and_ctrl_u_scroll_half_page() {
        let mut ed = make_multiline_editor(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
        ed.set_term_size(80, 6);

        ed.handle_key(ctrl_key('d'));
        assert_eq!(ed.cursor().row, 2);

        ed.handle_key(ctrl_key('u'));
        assert_eq!(ed.cursor().row, 0);
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
        ed.handle_key(key('d'));

        assert_eq!(ed.buffer().line_count(), 2);
        assert_eq!(ed.buffer().line(0), "line2");
        assert_eq!(ed.buffer().line(1), "line3");
    }

    #[test]
    fn test_single_d_waits_for_delete_operator() {
        let mut ed = make_multiline_editor(&["line1", "line2", "line3"]);

        ed.handle_key(key('d'));

        assert_eq!(ed.buffer().line_count(), 3);
        assert_eq!(ed.buffer().line(0), "line1");
    }

    #[test]
    fn test_count_prefix_dd_deletes_multiple_lines() {
        let mut ed = make_multiline_editor(&["line1", "line2", "line3"]);

        ed.handle_key(key('2'));
        ed.handle_key(key('d'));
        ed.handle_key(key('d'));

        assert_eq!(ed.buffer().line_count(), 1);
        assert_eq!(ed.buffer().line(0), "line3");
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

        ex(&mut ed, "xyz");
        assert!(!ed.should_quit());
        assert!(ed.message().is_some());
    }

    #[test]
    fn test_ex_line_address() {
        let mut ed = make_multiline_editor(&["one", "two", "three"]);

        ex(&mut ed, "3");
        assert_eq!(ed.cursor().row, 2);
        assert_eq!(ed.cursor().col, 0);
    }

    #[test]
    fn test_ex_last_line_address() {
        let mut ed = make_multiline_editor(&["one", "two", "three"]);

        ex(&mut ed, "$");
        assert_eq!(ed.cursor().row, 2);
        assert_eq!(ed.cursor().col, 0);
    }

    #[test]
    fn test_ex_invalid_line_address_reports_range_error() {
        let mut ed = make_multiline_editor(&["one", "two"]);

        ex(&mut ed, "9");
        assert_eq!(ed.cursor().row, 0);
        assert_eq!(ed.message(), Some("Invalid range: 9"));
    }

    #[test]
    fn test_ex_quit_bang_modified_buffer() {
        let mut ed = make_editor("");
        ed.handle_key(key('i'));
        ed.handle_key(key('x'));
        ed.handle_key(esc());

        ex(&mut ed, "q!");
        assert!(ed.should_quit());
    }

    #[test]
    fn test_ex_edit_bang_reloads_current_file() {
        let path = std::env::temp_dir().join(format!(
            "vivi-edit-bang-{}-{}.txt",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, "file text").unwrap();

        let mut ed = Editor::new(Buffer::from_file(&path).unwrap());
        ed.handle_key(key('A'));
        ed.handle_key(key('!'));
        ed.handle_key(esc());
        assert_eq!(ed.buffer().line(0), "file text!");

        ex(&mut ed, "e!");
        assert_eq!(ed.buffer().line(0), "file text");
        assert!(!ed.buffer().is_modified());

        let _ = std::fs::remove_file(path);
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

        // yank current line (line1) with yy
        ed.handle_key(key('y'));
        ed.handle_key(key('y'));
        // paste below
        ed.handle_key(key('p'));

        assert_eq!(ed.buffer().line_count(), 4);
        assert_eq!(ed.buffer().line(0), "line1");
        assert_eq!(ed.buffer().line(1), "line1");
    }

    #[test]
    fn test_yank_word_with_yw() {
        let mut ed = make_editor("hello world foo");

        // yank word forward
        ed.handle_key(key('y'));
        ed.handle_key(key('w'));

        // yank_register should contain "world" (cursor at 0, w moves to 6)
        match &ed.yank_register {
            YankRegister::Chars(s) => assert_eq!(s, "hello "),
            _ => panic!("expected Chars register"),
        }
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
    fn test_go_to_end_G_single_line() {
        let mut ed = make_editor("hello");
        ed.handle_key(key('G'));
        assert_eq!(ed.cursor().row, 0);
        assert!(ed.cursor().row < ed.buffer().line_count());
    }

    #[test]
    fn test_go_to_end_G_from_disk() {
        let path = std::env::temp_dir().join(format!(
            "vivi-g-test-{}-{}.txt",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, "line1\nline2\nline3\n").unwrap();
        let buf = Buffer::from_file(&path).unwrap();
        let mut ed = Editor::new(buf);
        let _ = std::fs::remove_file(&path);

        assert_eq!(ed.buffer().line_count(), 3);
        ed.handle_key(key('G'));
        assert_eq!(ed.cursor().row, 2);
        assert!(ed.cursor().row < ed.buffer().line_count());
    }

    #[test]
    fn test_go_to_end_G_from_disk_no_trailing_newline() {
        let path = std::env::temp_dir().join(format!(
            "vivi-g-test-{}-{}.txt",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::write(&path, "line1\nline2\nline3").unwrap();
        let buf = Buffer::from_file(&path).unwrap();
        let mut ed = Editor::new(buf);
        let _ = std::fs::remove_file(&path);

        assert_eq!(ed.buffer().line_count(), 3);
        ed.handle_key(key('G'));
        assert_eq!(ed.cursor().row, 2);
        assert!(ed.cursor().row < ed.buffer().line_count());
    }

    #[test]
    fn test_go_to_line_with_count_G() {
        let mut ed = make_multiline_editor(&["a", "b", "c", "d", "e"]);
        ed.handle_key(key('3'));
        ed.handle_key(key('G'));
        assert_eq!(ed.cursor().row, 2);
    }

    #[test]
    fn test_G_never_exceeds_line_count() {
        for n in 1..=20 {
            let lines: Vec<String> = (1..=n).map(|i| format!("line{i}")).collect();
            let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            let mut ed = make_multiline_editor(&line_refs);
            ed.handle_key(key('G'));
            assert!(
                ed.cursor().row < ed.buffer().line_count(),
                "G on {}-line file: cursor.row={} >= line_count={}",
                n,
                ed.cursor().row,
                ed.buffer().line_count()
            );
            assert_eq!(ed.cursor().row, n - 1);
        }
    }

    #[test]
    fn test_g_cursor_always_visible() {
        // Test edge cases with very small terminals
        use display_protocol::KeyEvent;

        for height in [4, 6, 8, 10, 24] {
            for n in 1..=15 {
                let mut buf = Buffer::new();
                buf.replace_range(0, 0, 0, "line0");
                for i in 1..n {
                    buf.insert_line(i, format!("line{}", i));
                }
                let mut ed = Editor::new(buf);
                ed.set_term_size(80, height);

                ed.handle_key(KeyEvent::char('G'));

                let text_height = (height as usize).saturating_sub(2);
                if text_height > 0 {
                    assert!(
                        ed.cursor().row >= ed.scroll().row,
                        "G on {}-line file, height={}: cursor.row={} < scroll.row={}",
                        n,
                        height,
                        ed.cursor().row,
                        ed.scroll().row
                    );
                    assert!(
                        ed.cursor().row < ed.scroll().row + text_height,
                        "G on {}-line file, height={}: cursor.row={} >= scroll.row+text_height={}",
                        n,
                        height,
                        ed.cursor().row,
                        ed.scroll().row + text_height
                    );
                }
            }
        }
    }

    #[test]
    fn test_go_to_top_gg() {
        let mut ed = make_multiline_editor(&["a", "b", "c"]);
        ed.handle_key(key('G')); // go to end first

        ed.handle_key(key('g'));
        ed.handle_key(key('g'));
        assert_eq!(ed.cursor().row, 0);
    }

    #[test]
    fn test_go_to_top_g_count() {
        let mut ed = make_multiline_editor(&["a", "b", "c", "d", "e"]);
        ed.handle_key(key('3'));
        ed.handle_key(key('g'));
        ed.handle_key(key('g'));
        assert_eq!(ed.cursor().row, 2);
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
